//! Broker administration is private to the server; agents can only publish their own topic.
use uuid::Uuid;
use serde_json::{json,Value};
use crate::postgres_db::Database;
pub fn enabled()->bool {std::env::var("MONITOR_ENV").as_deref()==Ok("production") || std::env::var_os("BROKER_ADMIN_PASSWORD_FILE").is_some()}
fn client()->anyhow::Result<(reqwest::Client,String,String)> {
 let base=std::env::var("BROKER_ADMIN_URL")?;
 let url=reqwest::Url::parse(&base)?;
 anyhow::ensure!(url.scheme()=="https" && url.username().is_empty() && url.password().is_none(),"Broker admin requires HTTPS without URL credentials");
 let secret=crate::startup_config::read_secret_file(&std::env::var("BROKER_ADMIN_PASSWORD_FILE")?,"BROKER_ADMIN_PASSWORD").map_err(|_|anyhow::anyhow!("Cannot read broker admin secret"))?;
 anyhow::ensure!(secret.len()>=32,"Broker administration secret must be random and at least 32 characters");
 let ca=std::fs::read(std::env::var("SSL_CERT_FILE")?)?;
 let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).redirect(reqwest::redirect::Policy::none()).add_root_certificate(reqwest::Certificate::from_pem(&ca)?).build()?;
 Ok((client,base.trim_end_matches('/').into(),secret))
}
pub fn validate_config()->anyhow::Result<()>{client().map(|_|())}
async fn request(method:reqwest::Method,path:&str,body:Option<Value>)->anyhow::Result<Value>{
 let (client,base,secret)=client()?;
 let deleting=method==reqwest::Method::DELETE;
 let mut request=client.request(method,format!("{base}/api/{path}")).basic_auth("monitor_identity",Some(secret));
 if let Some(body)=body {request=request.json(&body);}
 let response=request.send().await?;
 anyhow::ensure!(response.status().is_success() || (deleting && response.status()==reqwest::StatusCode::NOT_FOUND),"Broker identity operation failed");
 let bytes=response.bytes().await?;
 Ok(serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
pub async fn provision(id:Uuid,device:Uuid,token:&str)->anyhow::Result<()> {
 if !enabled(){return Ok(());}
 let user=format!("agent_{id}");
 request(reqwest::Method::PUT,&format!("users/{user}"),Some(json!({"password":token,"tags":[]}))).await?;
 request(reqwest::Method::PUT,&format!("permissions/%2F/{user}"),Some(json!({"configure":"^$","read":"^$","write":"^monitoring$"}))).await?;
 request(reqwest::Method::PUT,&format!("topic-permissions/%2F/{user}"),Some(json!({"exchange":"monitoring","write":format!("^monitoring\\.{}\\.(activity|inventory|heartbeat|health|usb|wifi|running_apps|input_heatmaps|security)$",device),"read":"^$"}))).await?;
 request(reqwest::Method::PUT,&format!("user-limits/{user}/max-connections"),Some(json!({"value":2}))).await?;
 request(reqwest::Method::PUT,&format!("user-limits/{user}/max-channels"),Some(json!({"value":4}))).await?;
 Ok(())
}
pub async fn remove(id:Uuid)->anyhow::Result<()> {
 if enabled(){request(reqwest::Method::DELETE,&format!("users/agent_{id}"),None).await?;}
 Ok(())
}
pub async fn sweep(db:&Database)->anyhow::Result<()> {
 // Delete expired/revoked users and orphan users from failed cross-service transactions.
 let mut lock=db.pool.begin().await?;
 let acquired=sqlx::query_scalar::<_,bool>("SELECT pg_try_advisory_xact_lock(734203404)").fetch_one(&mut *lock).await?;
 if !acquired{return Ok(());}
 let users=request(reqwest::Method::GET,"users",None).await?;
 for user in users.as_array().ok_or_else(||anyhow::anyhow!("Invalid broker user response"))? {
  let Some(id)=user["name"].as_str().and_then(|v|v.strip_prefix("agent_")).and_then(|v|Uuid::parse_str(v).ok()) else {continue};
  let active=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM agent_credentials WHERE credential_id=$1 AND revoked_at IS NULL AND expires_at>now())").bind(id).fetch_one(&mut *lock).await?;
  if !active {remove(id).await?;}
 }
 sqlx::query("DELETE FROM agent_enrollments WHERE expires_at<now()-interval '7 days'").execute(&mut *lock).await?;
 lock.commit().await?;
 Ok(())
}
pub fn spawn(db:Database){if !enabled(){return;}tokio::spawn(async move {let mut interval=tokio::time::interval(std::time::Duration::from_secs(60));loop {interval.tick().await;if sweep(&db).await.is_err(){tracing::error!("Broker credential reconciliation failed; consumer continues to reject revoked identities");}}});}

pub async fn validate_delivery(db:&Database,delivery:&lapin::message::Delivery,event:&Value)->anyhow::Result<bool>{
 if std::env::var("MONITOR_ENV").as_deref()!=Ok("production"){return Ok(true);}
 if delivery.data.len()>1048576{return Ok(false);}
 let user=delivery.properties.user_id().as_ref().map(|s|s.as_str()).unwrap_or("");
 let identity=if user=="monitor_server" {
  delivery.properties.headers().as_ref().and_then(|h|h.inner().get("x-monitor-identity")).and_then(|v|if let lapin::types::AMQPValue::LongString(v)=v{Some(v.to_string())}else{None}).unwrap_or_default()
 }else{user.to_string()};
 let Some(id)=identity.strip_prefix("agent_").and_then(|s|Uuid::parse_str(s).ok()) else{return Ok(false)};
 let device=sqlx::query_scalar::<_,Uuid>("SELECT device_id FROM agent_credentials WHERE credential_id=$1 AND revoked_at IS NULL AND expires_at>now()").bind(id).fetch_optional(&db.pool).await?;
 let Some(device)=device else{return Ok(false)};
 let matches=|v:&Value|v.as_str().and_then(|s|Uuid::parse_str(s).ok())==Some(device);
 if !matches(&event["device_id"]) || (event["payload"].get("device_id").is_some() && !matches(&event["payload"]["device_id"])) {return Ok(false);}
 if user!="monitor_server" && !delivery.routing_key.as_str().starts_with(&format!("monitoring.{device}.")){return Ok(false);}
 Ok(event["event_id"].as_str().and_then(|v|Uuid::parse_str(v).ok()).is_some() && event["boot_id"].as_str().and_then(|v|Uuid::parse_str(v).ok()).is_some() && event["sequence"].as_u64().is_some_and(|n|n>0))
}
pub fn retry_properties(delivery:&lapin::message::Delivery,mut headers:lapin::types::FieldTable)->lapin::BasicProperties {
 if std::env::var("MONITOR_ENV").as_deref()==Ok("production") {
  if let Some(user)=delivery.properties.user_id().as_ref().filter(|u|u.as_str().starts_with("agent_")){headers.insert("x-monitor-identity".into(),lapin::types::AMQPValue::LongString(user.as_str().into()));}
  delivery.properties.clone().with_headers(headers).with_user_id("monitor_server".into()).with_delivery_mode(2)
 }else{delivery.properties.clone().with_headers(headers).with_delivery_mode(2)}
}

use axum::{extract::{State,Path},http::{HeaderMap,StatusCode},response::{Response,IntoResponse},Json};
use std::sync::Arc;
use uuid::Uuid;
use serde_json::json;
use crate::{api::AppState,agent_identity::digest};
fn fail()->Response{(StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"Identity operation unavailable"}))).into_response()}
fn deny()->Response{(StatusCode::UNAUTHORIZED,Json(json!({"error":"Invalid or expired enrollment/credential"}))).into_response()}
pub fn random_token()->String {use rand::RngCore;let mut b=[0u8;32];rand::rngs::OsRng.fill_bytes(&mut b);b.iter().map(|b|format!("{b:02x}")).collect()}
fn valid_token(token:&str)->bool{token.len()==64 && token.bytes().all(|c|c.is_ascii_hexdigit())}
#[derive(serde::Deserialize)]
pub struct EnrollmentRequest { pub device_id:Uuid,pub code:String,pub new_token:String }
#[derive(serde::Deserialize)]
pub struct RenewRequest {pub new_token:String}
fn material(id:Uuid,device:Uuid,token:&str,expires:chrono::DateTime<chrono::Utc>)->Response {
 let endpoint=std::env::var("AGENT_BROKER_URL").unwrap_or_else(|_|"amqps://localhost:25671/%2F".into());
 let mut url=match reqwest::Url::parse(&endpoint){Ok(u) if u.scheme()=="amqps"=>u,_=>return fail()};
 if url.set_username(&format!("agent_{id}")).is_err() || url.set_password(Some(token)).is_err(){return fail();}
 (StatusCode::OK,[("cache-control","no-store")],Json(json!({"credential_id":id,"device_id":device,"expires_at":expires,"rabbitmq_url":url.as_str()}))).into_response()
}
pub async fn create(State(state):State<Arc<AppState>>,Path(device):Path<Uuid>,headers:HeaderMap)->Response {
 if !crate::broker_identity::enabled(){return fail();}
 let code=random_token();let id=Uuid::new_v4();
 let mut tx=match state.db.pool.begin().await{Ok(t)=>t,Err(_)=>return fail()};
 if !matches!(sqlx::query_scalar::<_,Uuid>("SELECT device_id FROM devices WHERE device_id=$1 FOR UPDATE").bind(device).fetch_optional(&mut *tx).await,Ok(Some(_))){return StatusCode::NOT_FOUND.into_response();}
 if sqlx::query("UPDATE agent_enrollments SET expires_at=now() WHERE device_id=$1 AND credential_id IS NULL AND expires_at>now()").bind(device).execute(&mut *tx).await.is_err(){return fail();}
 let expires=match sqlx::query_scalar::<_,chrono::DateTime<chrono::Utc>>("INSERT INTO agent_enrollments(enrollment_id,device_id,code_hash,expires_at) VALUES($1,$2,$3,now()+interval '1 hour') RETURNING expires_at").bind(id).bind(device).bind(digest(&code)).fetch_one(&mut *tx).await{Ok(v)=>v,Err(_)=>return fail()};
 let actor=headers.get("authorization").and_then(|h|h.to_str().ok()).and_then(|s|s.strip_prefix("Bearer ")).and_then(|t|state.auth.verify_token(t).ok()).map(|c|c.sub).unwrap_or_default();
 if sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($1,'agent.enrollment.created',$2,$3)").bind(actor).bind(device.to_string()).bind(id.to_string()).execute(&mut *tx).await.is_err(){return fail();}
 if tx.commit().await.is_err(){return fail();}
 (StatusCode::CREATED,[("cache-control","no-store")],Json(json!({"code":code,"device_id":device,"expires_at":expires}))).into_response()
}
pub async fn enroll(State(state):State<Arc<AppState>>,Json(body):Json<EnrollmentRequest>)->Response {
 if !crate::broker_identity::enabled(){return fail();}
 if !valid_token(&body.code)||!valid_token(&body.new_token){return deny();}
 let mut tx=match state.db.pool.begin().await{Ok(t)=>t,Err(_)=>return fail()};
 let row=sqlx::query_as::<_,(Uuid,Option<Uuid>)>("SELECT enrollment_id,credential_id FROM agent_enrollments WHERE code_hash=$1 AND device_id=$2 AND expires_at>now() FOR UPDATE").bind(digest(&body.code)).bind(body.device_id).fetch_optional(&mut *tx).await;
 let (enrollment,existing)=match row{Ok(Some(v))=>v,_=>return deny()};
 if let Some(id)=existing {
  let expiry=sqlx::query_scalar::<_,chrono::DateTime<chrono::Utc>>("SELECT expires_at FROM agent_credentials WHERE credential_id=$1 AND token_hash=$2 AND revoked_at IS NULL AND expires_at>now()").bind(id).bind(digest(&body.new_token)).fetch_optional(&mut *tx).await;
  return match expiry {Ok(Some(e))=>material(id,body.device_id,&body.new_token,e),_=>deny()};
 }
 let response=finish(&mut tx,body.device_id,&body.new_token,None,Some(enrollment)).await;
 if response.status().is_success() && tx.commit().await.is_err(){return fail();}
 response
}
pub async fn renew(State(state):State<Arc<AppState>>,headers:HeaderMap,Json(body):Json<RenewRequest>)->Response {
 if !crate::broker_identity::enabled(){return fail();}
 if !valid_token(&body.new_token){return deny();}
 let Some(old)=headers.get("x-agent-token").and_then(|v|v.to_str().ok()) else{return deny()};
 let mut tx=match state.db.pool.begin().await{Ok(t)=>t,Err(_)=>return fail()};
 let row=sqlx::query_as::<_,(Uuid,Uuid,bool)>("SELECT credential_id,device_id,expires_at<now()+interval '7 days' FROM agent_credentials WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>now() FOR UPDATE").bind(digest(old)).fetch_optional(&mut *tx).await;
 let (old_id,device,due)=match row{Ok(Some(v))=>v,_=>return deny()};
 let existing=sqlx::query_as::<_,(Uuid,chrono::DateTime<chrono::Utc>)>("SELECT credential_id,expires_at FROM agent_credentials WHERE predecessor=$1 AND token_hash=$2 AND revoked_at IS NULL AND expires_at>now()").bind(old_id).bind(digest(&body.new_token)).fetch_optional(&mut *tx).await;
 if let Ok(Some((id,e)))=existing {return material(id,device,&body.new_token,e);}
 if !due{return (StatusCode::CONFLICT,Json(json!({"error":"Renewal begins seven days before expiry"}))).into_response();}
 let response=finish(&mut tx,device,&body.new_token,Some(old_id),None).await;
 if response.status().is_success() && tx.commit().await.is_err(){return fail();}
 response
}
async fn finish(tx:&mut sqlx::Transaction<'_,sqlx::Postgres>,device:Uuid,token:&str,old:Option<Uuid>,enrollment:Option<Uuid>)->Response {
 // The caller commits only after broker provisioning succeeds.
 if sqlx::query("SELECT device_id FROM devices WHERE device_id=$1 FOR UPDATE").bind(device).execute(&mut **tx).await.is_err(){return fail();}
 let active=sqlx::query_scalar::<_,i64>("SELECT count(*) FROM agent_credentials WHERE device_id=$1 AND revoked_at IS NULL AND expires_at>now()").bind(device).fetch_one(&mut **tx).await;
 if !matches!(active,Ok(n) if n<2){return (StatusCode::CONFLICT,Json(json!({"error":"Revoke an active credential before enrollment or renewal"}))).into_response();}
 let id=Uuid::new_v4();
 let expires=match sqlx::query_scalar::<_,chrono::DateTime<chrono::Utc>>("INSERT INTO agent_credentials(credential_id,device_id,token_hash,expires_at,predecessor) VALUES($1,$2,$3,now()+interval '30 days',$4) RETURNING expires_at").bind(id).bind(device).bind(digest(token)).bind(old).fetch_one(&mut **tx).await{Ok(v)=>v,Err(_)=>return fail()};
 if let Some(old)=old{if sqlx::query("UPDATE agent_credentials SET expires_at=LEAST(expires_at,now()+interval '48 hours') WHERE credential_id=$1").bind(old).execute(&mut **tx).await.is_err(){return fail();}}
 if let Some(enrollment)=enrollment{if sqlx::query("UPDATE agent_enrollments SET credential_id=$2 WHERE enrollment_id=$1").bind(enrollment).bind(id).execute(&mut **tx).await.is_err(){return fail();}}
 if sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES('agent','agent.credential.provisioned',$1,$2)").bind(device.to_string()).bind(id.to_string()).execute(&mut **tx).await.is_err(){return fail();}
 if sqlx::query("SELECT pg_advisory_xact_lock(734203404)").execute(&mut **tx).await.is_err(){return fail();}
 if crate::broker_identity::provision(id,device,token).await.is_err(){return fail();}
 material(id,device,token,expires)
}

// Bounded global budget for enrollment/renewal, shared by /api and legacy aliases.
// No unbounded map indexed by attacker-controlled identifiers.
pub fn allow_attempt()->bool {
 static BUDGET:std::sync::OnceLock<std::sync::Mutex<(std::time::Instant,u32)>>=std::sync::OnceLock::new();
 let Ok(mut budget)=BUDGET.get_or_init(||std::sync::Mutex::new((std::time::Instant::now(),0))).lock() else{return false};
 if budget.0.elapsed()>=std::time::Duration::from_secs(60){*budget=(std::time::Instant::now(),0);}
 if budget.1>=120{return false;}budget.1+=1;true
}

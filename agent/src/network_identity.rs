//! Persisted, encrypted network credentials; renewal never changes device ID or outbox key.
use serde::{Serialize,Deserialize};
use chrono::{DateTime,Utc};
use uuid::Uuid;
use sha2::{Digest,Sha256};
use aes_gcm::{Aes256Gcm,Nonce,aead::{Aead,KeyInit}};
type Error=Box<dyn std::error::Error+Send+Sync>;
pub fn production()->bool{std::env::var("MONITOR_ENV").as_deref()==Ok("production")}
#[derive(Serialize,Deserialize)]
struct Snapshot {device_id:Uuid,token:String,rabbitmq_url:String,expires_at:Option<DateTime<Utc>>,pending_token:Option<String>,#[serde(default)] enrollment_hash:Option<String>,#[serde(default)] pending_enrollment_hash:Option<String>}
fn path()->std::path::PathBuf{crate::runtime::data_dir().join("network-identity.json")}
fn lock()->Result<std::fs::File,Error>{
 std::fs::create_dir_all(crate::runtime::data_dir())?;
 let mut options=std::fs::OpenOptions::new();options.read(true).write(true).create(true).truncate(false);
 #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
 let file=options.open(crate::runtime::data_dir().join("network-identity.lock"))?;
 file.try_lock().map_err(|_|"Another credential operation is running")?;Ok(file)
}
fn cipher()->Result<Aes256Gcm,Error>{let key=std::env::var("AGENT_OFFLINE_CACHE_KEY")?;Ok(Aes256Gcm::new_from_slice(&Sha256::digest(key.as_bytes()))?)}
fn load()->Result<Snapshot,Error>{
 if std::fs::metadata(path())?.len()>65536{return Err("Network credential file exceeds limit".into());}
 let data:serde_json::Value=serde_json::from_slice(&std::fs::read(path())?)?;
 let nonce:Vec<u8>=serde_json::from_value(data["nonce"].clone())?;
 if nonce.len()!=12{return Err("Invalid identity nonce".into());}
 let ciphertext:Vec<u8>=serde_json::from_value(data["ciphertext"].clone())?;
 let bytes=cipher()?.decrypt(Nonce::from_slice(&nonce),ciphertext.as_ref()).map_err(|_|"Cannot decrypt network credentials; restore the original cache key")?;
 Ok(serde_json::from_slice(&bytes)?)
}
fn save(value:&Snapshot)->Result<(),Error>{
 use rand::RngCore;let mut nonce=[0u8;12];rand::rngs::OsRng.fill_bytes(&mut nonce);
 let bytes=serde_json::to_vec(value)?;
 let ciphertext=cipher()?.encrypt(Nonce::from_slice(&nonce),bytes.as_ref()).map_err(|_|"Cannot encrypt credentials")?;
 crate::persistent_file::write_json(&path(),&serde_json::json!({"nonce":nonce,"ciphertext":ciphertext}))?;Ok(())
}
pub fn token(fallback:&str)->String{if production(){load().map(|v|v.token).unwrap_or_default()}else{fallback.into()}}
pub fn rabbit_url(fallback:&str)->String{if production(){load().map(|v|v.rabbitmq_url).unwrap_or_default()}else{fallback.into()}}
pub fn client()->Result<reqwest::Client,Error>{
 let mut builder=reqwest::Client::builder().timeout(std::time::Duration::from_secs(15)).redirect(reqwest::redirect::Policy::none());
 if production(){builder=builder.https_only(true);if let Ok(path)=std::env::var("SSL_CERT_FILE"){builder=builder.add_root_certificate(reqwest::Certificate::from_pem(&std::fs::read(path)?)?);}}
 Ok(builder.build()?)
}
fn random_token()->String{use rand::RngCore;let mut b=[0u8;32];rand::rngs::OsRng.fill_bytes(&mut b);b.iter().map(|v|format!("{v:02x}")).collect()}
pub async fn initialize(device:Uuid)->Result<(),Error>{
 if !production()||crate::runtime::is_session(){return Ok(());}
 {let _lock=lock()?;
 if path().exists(){if load()?.device_id!=device{return Err("Network identity belongs to a different device".into());}}
 else{save(&Snapshot{device_id:device,token:String::new(),rabbitmq_url:String::new(),expires_at:None,pending_token:Some(random_token()),enrollment_hash:None,pending_enrollment_hash:None})?;}
 }
 if refresh().await.is_err(){tracing::warn!("Enrollment/renewal unavailable; encrypted outbox retained, retry scheduled");}
 Ok(())
}
async fn refresh()->Result<bool,Error>{
 let _lock=lock()?;
 let mut snapshot=load()?;
 let code=std::env::var("AGENT_ENROLLMENT_CODE_FILE").ok().and_then(|p|std::fs::read_to_string(p).ok()).map(|v|v.trim().to_owned());
 let code_hash=code.as_ref().map(|v|format!("{:x}",Sha256::digest(v.as_bytes())));
 let enrolling=code_hash.is_some() && code_hash!=snapshot.enrollment_hash;
 if enrolling && code_hash!=snapshot.pending_enrollment_hash {snapshot.pending_token=Some(random_token());snapshot.pending_enrollment_hash=code_hash.clone();save(&snapshot)?;}
 if !enrolling && snapshot.pending_token.is_none() && snapshot.expires_at.is_some_and(|e|e>Utc::now()+chrono::Duration::days(6)){return Ok(false);}
 if snapshot.pending_token.is_none(){snapshot.pending_token=Some(random_token());save(&snapshot)?;}
 let pending=snapshot.pending_token.clone().ok_or("Missing pending token")?;
 let base=std::env::var("AGENT_SERVER_URL")?;
 let request=if enrolling || snapshot.token.is_empty(){
  let code=code.ok_or("Enrollment code required")?;
  client()?.post(format!("{}/api/agent/enroll",base.trim_end_matches('/'))).json(&serde_json::json!({"device_id":snapshot.device_id,"code":code.trim(),"new_token":pending}))
 }else{
  client()?.post(format!("{}/api/agent/renew",base.trim_end_matches('/'))).header("x-agent-token",&snapshot.token).json(&serde_json::json!({"new_token":pending}))
 };
 let response=request.send().await?;
 if !response.status().is_success(){return Err("Network credential operation rejected; check expiry, revocation or enrollment code".into());}
 let data:serde_json::Value=response.json().await?;
 if data["device_id"].as_str().and_then(|v|Uuid::parse_str(v).ok())!=Some(snapshot.device_id){return Err("Enrollment identity mismatch".into());}
 let url=data["rabbitmq_url"].as_str().ok_or("Missing broker URL")?;
 let parsed=reqwest::Url::parse(url)?;
 if parsed.scheme()!="amqps" || parsed.password()!=Some(pending.as_str()){return Err("Invalid broker credential response".into());}
 snapshot.expires_at=Some(serde_json::from_value(data["expires_at"].clone())?);
 snapshot.token=pending;snapshot.rabbitmq_url=url.into();snapshot.pending_token=None;
 if enrolling{snapshot.enrollment_hash=code_hash;snapshot.pending_enrollment_hash=None;}save(&snapshot)?;
 Ok(true)
}
pub fn spawn(context:std::sync::Arc<crate::tasks::TaskContext>)->tokio::task::JoinHandle<()>{tokio::spawn(async move{
 let mut interval=tokio::time::interval(std::time::Duration::from_secs(60));
 loop{interval.tick().await;match refresh().await{Ok(true)=>{*context.publisher.write().await=None;tracing::info!("Network credentials renewed; reconnecting without changing device identity or outbox");},Ok(false)=>{},Err(_)=>tracing::warn!("Network credential refresh failed; pending events remain encrypted locally")}}
})}

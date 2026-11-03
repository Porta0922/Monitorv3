//! HTTP device credentials. Tokens are random; only SHA-256 digests are persisted.
use axum::{extract::{State, Path}, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, Json, Router, routing::{get, delete}};
use std::sync::Arc;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use crate::api::AppState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Identity { Legacy, Device(Uuid) }
pub fn digest(token: &str) -> String { format!("{:x}", Sha256::digest(token.as_bytes())) }
pub async fn authenticate(state: &AppState, headers: &HeaderMap) -> Option<Identity> {
    let token=headers.get("x-agent-token")?.to_str().ok()?;
    if token.is_empty() || token.len()>256 { return None; }
    // Compatibility is explicitly restricted to non-production installations.
    if std::env::var("MONITOR_ENV").as_deref()!=Ok("production") {
        let expected=state.config.agent_auth_token.as_bytes();
        if !expected.is_empty() && token.len()==expected.len() && token.bytes().zip(expected).fold(0u8,|d,(a,b)|d|(a^b))==0 { return Some(Identity::Legacy); }
    }
    let id=sqlx::query_scalar::<_,Uuid>("SELECT device_id FROM agent_credentials WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>now()")
        .bind(digest(token)).fetch_optional(&state.db.pool).await.ok()??;
    Some(Identity::Device(id))
}
pub fn matches_device(identity: Identity, value: Option<&str>) -> bool {
    match identity { Identity::Legacy=>true, Identity::Device(id)=>value.and_then(|v|Uuid::parse_str(v).ok())==Some(id) }
}
pub fn router()->Router<Arc<AppState>> {
    Router::new().route("/enrollment/:device_id",axum::routing::post(crate::agent_enrollment::create)).route("/:device_id",get(list).post(issue)).route("/:device_id/:credential_id",delete(revoke))
}
fn actor(state:&AppState, headers:&HeaderMap)->String {
 headers.get("authorization").and_then(|h|h.to_str().ok()).and_then(|v|v.strip_prefix("Bearer ")).and_then(|t|state.auth.verify_token(t).ok()).map(|c|c.sub).unwrap_or_else(||"unknown".into())
}
fn failure()->Response { (StatusCode::SERVICE_UNAVAILABLE,Json(serde_json::json!({"error":"Credential operation unavailable"}))).into_response() }
async fn list(State(state):State<Arc<AppState>>,Path(device):Path<Uuid>)->Response {
    match sqlx::query_scalar::<_,serde_json::Value>("SELECT jsonb_build_object('credential_id',credential_id,'device_id',device_id,'created_at',created_at,'expires_at',expires_at,'revoked_at',revoked_at) FROM agent_credentials WHERE device_id=$1 ORDER BY created_at DESC LIMIT 100")
        .bind(device).fetch_all(&state.db.pool).await {
        Ok(rows)=>Json(serde_json::json!({"credentials":rows,"broker_managed":crate::broker_identity::enabled()})).into_response(),Err(_)=>failure()
    }
}
#[derive(serde::Deserialize)]
struct Issue { valid_days: u32 }
async fn issue(State(state):State<Arc<AppState>>,Path(device):Path<Uuid>,headers:HeaderMap,Json(body):Json<Issue>)->Response {
    if !(1..=90).contains(&body.valid_days) { return (StatusCode::BAD_REQUEST,Json(serde_json::json!({"error":"valid_days must be 1..90"}))).into_response(); }
    use rand::RngCore;
    let mut bytes=[0u8;32];rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token=bytes.iter().map(|b|format!("{b:02x}")).collect::<String>();
    let id=Uuid::new_v4();
    let mut tx=match state.db.pool.begin().await {Ok(tx)=>tx,Err(_)=>return failure()};
    let exists=sqlx::query_scalar::<_,Uuid>("SELECT device_id FROM devices WHERE device_id=$1 FOR UPDATE").bind(device).fetch_optional(&mut *tx).await;
    if !matches!(exists,Ok(Some(_))) {return (StatusCode::NOT_FOUND,Json(serde_json::json!({"error":"Device unavailable"}))).into_response();}
    let active=sqlx::query_scalar::<_,i64>("SELECT count(*) FROM agent_credentials WHERE device_id=$1 AND revoked_at IS NULL AND expires_at>now()") .bind(device).fetch_one(&mut *tx).await;
    match active {Ok(n) if n<2=>{},Ok(_)=>return (StatusCode::CONFLICT,Json(serde_json::json!({"error":"Revoke an active credential before issuing another"}))).into_response(),Err(_)=>return failure()}
    let expires=sqlx::query_scalar::<_,chrono::DateTime<chrono::Utc>>("INSERT INTO agent_credentials(credential_id,device_id,token_hash,expires_at) VALUES($1,$2,$3,now()+make_interval(days=>$4)) RETURNING expires_at")
        .bind(id).bind(device).bind(digest(&token)).bind(body.valid_days as i32).fetch_one(&mut *tx).await;
    let expires=match expires {Ok(v)=>v,Err(_)=>return failure()};
    if sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($3,'agent.credential.issued',$1,$2)")
        .bind(device.to_string()).bind(format!("credential_id={id}")).bind(actor(&state,&headers)).execute(&mut *tx).await.is_err() {return failure();}
    if sqlx::query("SELECT pg_advisory_xact_lock(734203404)").execute(&mut *tx).await.is_err(){return failure();}
    if crate::broker_identity::provision(id,device,&token).await.is_err(){return failure();}
    if tx.commit().await.is_err(){return failure();}
    (StatusCode::CREATED,[("cache-control","no-store")],Json(serde_json::json!({"credential_id":id,"device_id":device,"token":token,"expires_at":expires}))).into_response()
}
async fn revoke(State(state):State<Arc<AppState>>,Path((device,id)):Path<(Uuid,Uuid)>,headers:HeaderMap)->Response {
    let mut tx=match state.db.pool.begin().await {Ok(tx)=>tx,Err(_)=>return failure()};
    let updated=sqlx::query("UPDATE agent_credentials SET revoked_at=COALESCE(revoked_at,now()) WHERE device_id=$1 AND credential_id=$2").bind(device).bind(id).execute(&mut *tx).await;
    match updated {Ok(r) if r.rows_affected()==0=>return StatusCode::NOT_FOUND.into_response(),Err(_)=>return failure(),_=>{}}
    if sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($3,'agent.credential.revoked',$1,$2)").bind(device.to_string()).bind(format!("credential_id={id}")).bind(actor(&state,&headers)).execute(&mut *tx).await.is_err(){return failure();}
    if tx.commit().await.is_err(){return failure();}
    if crate::broker_identity::remove(id).await.is_err(){return (StatusCode::ACCEPTED,Json(serde_json::json!({"revoked":true,"broker_disconnect":"pending_retry"}))).into_response();}
    StatusCode::NO_CONTENT.into_response()
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn identity_cannot_select_another_device(){let a=Uuid::new_v4();let b=Uuid::new_v4();assert!(matches_device(Identity::Device(a),Some(&a.to_string())));assert!(!matches_device(Identity::Device(a),Some(&b.to_string())));assert!(!matches_device(Identity::Device(a),None));assert!(!matches_device(Identity::Device(a),Some("invalid")));}
 #[test] fn stable_digest_not_plaintext(){assert_eq!(digest("abc"),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");}
}

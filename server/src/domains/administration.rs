use axum::{extract::{Path,State},http::{HeaderMap,StatusCode},routing::get,Json,Router};
use serde_json::{json,Value};
use std::sync::Arc;
use crate::api::AppState;
type Response=(StatusCode,Json<Value>);
#[cfg(test)]
mod privacy_tests {
    use super::*;
    #[test]
    fn privacy_schema_accepts_legacy_and_rejects_partial_or_unbounded_rules() {
        let mut policy=serde_json::to_value(crate::domains::agent::routes::default_policy()).unwrap();
        assert!(valid(&policy));
        policy["privacy_title_filters"]=json!([""]);
        assert!(!valid(&policy));
        policy["privacy_title_filters"]=json!(["confidencial"]);
        assert!(valid(&policy));
        policy.as_object_mut().unwrap().remove("privacy_hide_titles");
        assert!(!valid(&policy));
        for key in ["privacy_hide_paths","privacy_excluded_apps","privacy_title_filters"] {policy.as_object_mut().unwrap().remove(key);}
        assert!(valid(&policy));
    }
}
pub fn router()->Router<Arc<AppState>> {Router::new().route("/policy/:scope",get(read_policy).put(save_policy)).route("/versions",get(versions)).route("/accounts",get(accounts))}
pub async fn effective(state:&AppState,device:Option<&str>)->Result<Value,sqlx::Error> {
    let fallback=serde_json::to_value(crate::domains::agent::routes::default_policy()).expect("Policy serialization");
    let stored=sqlx::query_scalar::<_,Value>("SELECT policy FROM administration_policy WHERE policy IS NOT NULL AND (scope='global' OR scope=$1) ORDER BY (scope=$1) DESC LIMIT 1")
        .bind(device.unwrap_or("global")).fetch_optional(&state.db.pool).await?;
    let mut policy = stored.unwrap_or_else(|| fallback.clone());
    if let Some(object) = policy.as_object_mut() {
        for key in ["privacy_hide_titles", "privacy_hide_paths", "privacy_excluded_apps", "privacy_title_filters"] {
            object.entry(key).or_insert_with(|| fallback[key].clone());
        }
    }
    Ok(policy)
}
fn valid(policy:&Value)->bool {
    let fields=[("window_activity_interval_secs",2,60),("heartbeat_interval_secs",1,86400),("usb_detector_interval_secs",1,86400),("usb_copy_interval_secs",1,86400),("wifi_interval_secs",1,86400),("running_apps_interval_secs",1,86400),("inventory_interval_days",1,365),("heatmap_interval_secs",1,86400),("resource_logger_interval_secs",1,60),("osquery_scheduler_seconds",0,86400),("idle_threshold_seconds",1,86400),("activity_heartbeat_seconds",1,3600)];
    let privacy_keys = ["privacy_hide_titles", "privacy_hide_paths", "privacy_excluded_apps", "privacy_title_filters"];
    let privacy_present = privacy_keys.iter().any(|key| policy.get(*key).is_some());
    if privacy_present {
        if !policy["privacy_hide_titles"].is_boolean() || !policy["privacy_hide_paths"].is_boolean() {return false;}
        for key in ["privacy_excluded_apps", "privacy_title_filters"] {
            if !policy[key].as_array().is_some_and(|items| items.len()<=100 && items.iter().all(|v| v.as_str().is_some_and(|s| !s.trim().is_empty() && s.len()<=256 && !s.chars().any(char::is_control)))) {return false;}
        }
    }
    if policy.as_object().map(|p|p.len())!=Some(if privacy_present {19} else {15}) || !policy["monitoring_paused"].is_boolean() {return false;}
    if fields.iter().any(|(key,min,max)|!policy[*key].as_u64().is_some_and(|v|v>=*min && v<=*max)) {return false;}
    let monitors=["window_activity","heartbeat","usb_detector","usb_copy","wifi_history","running_apps","inventory","heatmap","resource_logger","security_osquery","session_input"];
    if let (Some(enabled),Some(disabled))=(policy["enabled_monitors"].as_array(),policy["disabled_monitors"].as_array()) {
        if enabled.iter().any(|v|disabled.contains(v)){return false;}
        for list in [enabled,disabled]{let unique:std::collections::HashSet<_>=list.iter().filter_map(Value::as_str).collect();if unique.len()!=list.len(){return false;}}
    }
    ["enabled_monitors","disabled_monitors"].iter().all(|key|policy[*key].as_array().is_some_and(|a|a.len()<=monitors.len() && a.iter().all(|v|v.as_str().is_some_and(|s|monitors.contains(&s)))))
}
async fn scope_exists(state:&AppState,scope:&str)->Result<bool,sqlx::Error> {
    if scope=="global" {return Ok(true);}
    let Ok(id)=uuid::Uuid::parse_str(scope) else {return Ok(false);};
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE device_id=$1)").bind(id).fetch_one(&state.db.pool).await
}
async fn read_policy(State(state):State<Arc<AppState>>,Path(scope):Path<String>)->Response {
    let result:Result<Value,sqlx::Error>=async {
        if !scope_exists(&state,&scope).await? {return Ok(Value::Null);}
        let row=sqlx::query_scalar::<_,Value>("SELECT to_jsonb(p) FROM administration_policy p WHERE scope=$1").bind(&scope).fetch_optional(&state.db.pool).await?;
        let policy=effective(&state,Some(&scope)).await?;
        let fingerprint={use sha2::Digest;format!("{:x}",sha2::Sha256::digest(serde_json::to_vec(&policy).expect("Policy serialization")))};
        let observed=if let Ok(id)=uuid::Uuid::parse_str(&scope) {
            sqlx::query_scalar::<_,Value>("SELECT jsonb_build_object('fingerprint',payload->'policy_fingerprint','observed_at',observed_at,'stale',observed_at<now()-interval '900 seconds') FROM device_telemetry_state WHERE device_id=$1 AND kind='health'").bind(id).fetch_optional(&state.db.pool).await?
        } else {None};
        Ok(json!({"scope":scope,"revision":row.as_ref().map(|r|r["revision"].clone()).unwrap_or(json!(0)),"override":row.as_ref().is_some_and(|r|!r["policy"].is_null()),"policy":policy,"fingerprint":fingerprint,"observed":observed,"updated_at":row.as_ref().map(|r|r["updated_at"].clone())}))
    }.await;
    match result {Ok(Value::Null)=>(StatusCode::NOT_FOUND,Json(json!({"error":"Unknown scope"}))),Ok(value)=>(StatusCode::OK,Json(value)),Err(e)=>failure(e)}
}
#[derive(serde::Deserialize)]
struct Save {revision:i64,policy:Option<Value>,reason:String}
async fn save_policy(State(state):State<Arc<AppState>>,Path(scope):Path<String>,headers:HeaderMap,Json(body):Json<Save>)->Response {
    if body.revision<0 || body.revision==i64::MAX || body.reason.trim().is_empty() || body.reason.len()>2000 || body.policy.as_ref().is_some_and(|p|!valid(p)) {return (StatusCode::BAD_REQUEST,Json(json!({"error":"Invalid complete policy, revision or reason"})));}
    let actor=headers.get("authorization").and_then(|v|v.to_str().ok()).and_then(|v|v.strip_prefix("Bearer ")).and_then(|token|state.auth.verify_token(token).ok());
    let Some(actor)=actor.filter(|a|a.role==crate::auth::Role::Admin) else {return (StatusCode::FORBIDDEN,Json(json!({"error":"Administrator required"})));};
    let result:Result<i64,sqlx::Error>=async {
        if !scope_exists(&state,&scope).await? {return Ok(-2);}
        let mut tx=state.db.pool.begin().await?;
        sqlx::query("SET LOCAL lock_timeout = '5s'").execute(&mut *tx).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(734203403)").execute(&mut *tx).await?;
        let revision:Option<i64>=sqlx::query_scalar("SELECT revision FROM administration_policy WHERE scope=$1").bind(&scope).fetch_optional(&mut *tx).await?;
        if revision.unwrap_or(0)!=body.revision {return Ok(-1);}
        let next=body.revision+1;
        sqlx::query("INSERT INTO administration_policy(scope,policy,revision,updated_by) VALUES($1,$2,$3,$4) ON CONFLICT(scope) DO UPDATE SET policy=EXCLUDED.policy,revision=EXCLUDED.revision,updated_by=EXCLUDED.updated_by,updated_at=now()")
            .bind(&scope).bind(body.policy).bind(next).bind(&actor.sub).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($1,'policy.saved',$2,$3)").bind(&actor.sub).bind(&scope).bind(json!({"revision":next,"reason":body.reason}).to_string()).execute(&mut *tx).await?;
        tx.commit().await?;Ok(next)
    }.await;
    match result {Ok(-1)=>(StatusCode::CONFLICT,Json(json!({"error":"Policy changed; reload before saving"}))),Ok(-2)=>(StatusCode::NOT_FOUND,Json(json!({"error":"Unknown scope"}))),Ok(revision)=>(StatusCode::OK,Json(json!({"revision":revision,"status":"saved_waiting_for_agent"}))),Err(e)=>failure(e)}
}
async fn versions(State(state):State<Arc<AppState>>)->Response {
    match sqlx::query_scalar::<_,Value>("SELECT jsonb_build_object('versions',COALESCE((SELECT jsonb_agg(to_jsonb(v)) FROM (SELECT COALESCE(version,'unknown') AS version,count(*) AS devices FROM devices GROUP BY version ORDER BY version) v),'[]'::jsonb),'targeted',(SELECT count(*) FROM devices WHERE desired_version IS NOT NULL),'matching',(SELECT count(*) FROM devices WHERE desired_version IS NOT NULL AND version=desired_version),'different',(SELECT count(*) FROM devices WHERE desired_version IS NOT NULL AND version IS DISTINCT FROM desired_version))").fetch_one(&state.db.pool).await {
        Ok(value)=>(StatusCode::OK,Json(value)),Err(e)=>failure(e)
    }
}
fn failure(error:sqlx::Error)->Response {tracing::error!(%error,"Administration unavailable");(StatusCode::INTERNAL_SERVER_ERROR,Json(json!({"error":"Administration unavailable"})))}

async fn accounts(State(state):State<Arc<AppState>>,headers:HeaderMap)->Response {
    let admin=headers.get("authorization").and_then(|v|v.to_str().ok()).and_then(|v|v.strip_prefix("Bearer ")).and_then(|token|state.auth.verify_token(token).ok()).is_some_and(|p|p.role==crate::auth::Role::Admin);
    if !admin { return (StatusCode::FORBIDDEN,Json(json!({"error":"Administrator required"}))); }
    match crate::auth::configured_accounts() {
        Ok(Some(accounts)) => (StatusCode::OK,Json(json!({"source":"managed_file","accounts":accounts.iter().map(|a|json!({"username":a.username,"role":a.role,"disabled":a.disabled,"mfa_enabled":a.totp_secret.is_some()})).collect::<Vec<_>>()}))),
        Ok(None) => (StatusCode::OK,Json(json!({"source":"bootstrap","accounts":[]}))),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"Account configuration unavailable"}))),
    }
}

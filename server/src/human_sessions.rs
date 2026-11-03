//! Opaque browser sessions. No reusable human bearer token is exposed to JavaScript.
use axum::{extract::{State,Path,Extension},http::{HeaderMap,StatusCode},response::{IntoResponse,Response},routing::{get,post,delete},Json,Router};
use std::sync::Arc;
use serde_json::json;
use uuid::Uuid;
use crate::{api::AppState,auth::{Role,Claims,Account},agent_identity::digest};
use hmac::{Hmac,Mac};
pub fn production()->bool{std::env::var("MONITOR_ENV").as_deref()==Ok("production")}
pub fn mac(key:&[u8],data:&[u8])->String {let mut mac=Hmac::<sha2::Sha256>::new_from_slice(key).expect("HMAC accepts any key");mac.update(data);mac.finalize().into_bytes().iter().map(|b|format!("{b:02x}")).collect()}
pub fn equal(a:&str,b:&str)->bool{a.len()==b.len() && a.bytes().zip(b.bytes()).fold(0u8,|d,(a,b)|d|(a^b))==0}
pub fn totp_key(secret:&str)->Result<Vec<u8>,String>{let key=data_encoding::BASE32_NOPAD.decode(secret.as_bytes()).map_err(|_|"Invalid base32")?;if !(20..=64).contains(&key.len()){return Err("TOTP key must contain 20..64 bytes".into());}Ok(key)}
pub fn totp_step(secret:&str,code:&str,now:i64)->Option<i64>{
 if code.len()!=6 || !code.bytes().all(|b|b.is_ascii_digit()){return None;}
 let key=totp_key(secret).ok()?;
 for step in [now/30,now/30-1,now/30+1] {let mut h=Hmac::<sha1::Sha1>::new_from_slice(&key).ok()?;h.update(&(step as u64).to_be_bytes());let bytes=h.finalize().into_bytes();let i=(bytes[19]&15) as usize;let value=(u32::from_be_bytes(bytes[i..i+4].try_into().ok()?)&0x7fff_ffff)%1_000_000;if equal(code,&format!("{value:06}")){return Some(step);}}
 None
}
pub fn version(account:&Account)->String{digest(&format!("{}\0{}\0{}\0{}",account.username,account.password_hash,account.totp_secret.as_deref().unwrap_or(""),account.session_epoch))}
fn minimum(a:Role,b:Role)->Role{match(a,b){(Role::Viewer,_)|(_,Role::Viewer)=>Role::Viewer,(Role::Operator,_)|(_,Role::Operator)=>Role::Operator,_=>Role::Admin}}
pub fn cookie_name()->&'static str{if production(){"__Host-monitor_session"}else{"monitor_session"}}
pub fn cookie(headers:&HeaderMap)->Option<String>{let name=format!("{}=",cookie_name());let mut values=headers.get_all("cookie").iter().filter_map(|v|v.to_str().ok()).flat_map(|v|v.split(';')).filter_map(|v|v.trim().strip_prefix(&name));let value=values.next()?;if values.next().is_some() || value.len()!=64 || !value.bytes().all(|b|b.is_ascii_hexdigit()){return None;}Some(value.into())}
fn set_cookie(value:&str,age:u32)->String{format!("{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",cookie_name(),value,age,if production(){"; Secure"}else{""})}
#[derive(Clone)]
pub struct Session {pub id:Uuid,pub claims:Claims,pub csrf:String}
pub async fn authenticate(state:&AppState,headers:&HeaderMap)->Result<Option<Session>,()> {
 let Some(token)=cookie(headers) else{return Ok(None)};
 let row=sqlx::query_as::<_,(Uuid,String,String,String,i64)>("SELECT session_id,username,issued_role,account_version,extract(epoch from expires_at)::bigint FROM human_sessions WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>now() AND last_seen>now()-interval '30 minutes'").bind(digest(&token)).fetch_optional(&state.db.pool).await.map_err(|_|())?;
 let Some((id,user,role,stored,expires))=row else{return Ok(None)};
 let issued:Role=serde_json::from_value(json!(role)).map_err(|_|())?;
 let current=match crate::auth::configured_accounts().map_err(|_|())? {
  Some(accounts)=>accounts.into_iter().find(|a|a.username==user && !a.disabled).filter(|a|equal(&version(a),&stored)).map(|a|a.role),
  None if !production()=>{let name=std::env::var("ADMIN_USERNAME").unwrap_or_else(|_|"admin".into());let password=std::env::var("ADMIN_PASSWORD").unwrap_or_default();if !password.is_empty() && name==user && stored==digest(&format!("bootstrap:{name}:{password}")){Some(Role::Admin)}else{None}},
  _=>return Err(())
 };
 let Some(current)=current else{return Ok(None)};
 let effective=minimum(issued,current);
 let role_text=serde_json::to_value(effective).map_err(|_|())?.as_str().ok_or(())?.to_owned();
 let stored_role=sqlx::query_scalar::<_,String>("UPDATE human_sessions SET last_seen=now(),issued_role=CASE WHEN issued_role='viewer' OR $2='viewer' THEN 'viewer' WHEN issued_role='operator' OR $2='operator' THEN 'operator' ELSE 'admin' END WHERE session_id=$1 AND revoked_at IS NULL AND expires_at>now() AND last_seen>now()-interval '30 minutes' RETURNING issued_role").bind(id).bind(role_text).fetch_optional(&state.db.pool).await.map_err(|_|())?;
 let Some(stored_role)=stored_role else{return Ok(None)};
 let effective:Role=serde_json::from_value(json!(stored_role)).map_err(|_|())?;
 Ok(Some(Session{id,claims:Claims{sub:user,role:effective,iat:chrono::Utc::now().timestamp(),exp:expires},csrf:state.auth.csrf(&token)}))
}
pub fn origin_allowed(headers:&HeaderMap)->bool {
 if headers.get("sec-fetch-site").and_then(|h|h.to_str().ok())==Some("cross-site"){return false;}
 let Some(origin)=headers.get("origin").and_then(|h|h.to_str().ok()) else{return true};
 let expected=if production(){std::env::var("MONITOR_PUBLIC_ORIGIN").ok()}else{headers.get("host").and_then(|v|v.to_str().ok()).map(|h|format!("http://{h}"))};
 expected.is_some_and(|e|e==origin)
}
pub fn csrf_valid(headers:&HeaderMap,session:&Session)->bool {origin_allowed(headers)&&headers.get("x-csrf-token").and_then(|v|v.to_str().ok()).is_some_and(|v|equal(v,&session.csrf))}
fn response_error(code:StatusCode,message:&str)->Response{(code,[("cache-control","no-store")],Json(json!({"error":message}))).into_response()}
pub async fn audit(state:&AppState,actor:&str,action:&str,target:&str,result:&str)->Result<(),sqlx::Error>{sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($1,$2,$3,$4)").bind(actor.chars().take(255).collect::<String>()).bind(action.chars().take(128).collect::<String>()).bind(target.chars().take(255).collect::<String>()).bind(json!({"result":result}).to_string()).execute(&state.db.pool).await?;Ok(())}
fn budget()->bool{static B:std::sync::OnceLock<std::sync::Mutex<(std::time::Instant,u32)>>=std::sync::OnceLock::new();let Ok(mut b)=B.get_or_init(||std::sync::Mutex::new((std::time::Instant::now(),0))).lock()else{return false};if b.0.elapsed().as_secs()>=60{*b=(std::time::Instant::now(),0);}if b.1>=120{return false;}b.1+=1;true}
#[derive(serde::Deserialize)]
pub struct Login {username:String,password:String,#[serde(default)] otp:String}
pub async fn login(State(state):State<Arc<AppState>>,headers:HeaderMap,Json(body):Json<Login>)->Response {
 if !origin_allowed(&headers)||headers.get("x-monitor-client").and_then(|v|v.to_str().ok())!=Some("dashboard"){return response_error(StatusCode::FORBIDDEN,"Request origin rejected");}
 if body.username.len()>100 || body.password.len()>4096 || body.otp.len()>16{return response_error(StatusCode::UNAUTHORIZED,"Invalid credentials");}
 if !budget(){return (StatusCode::TOO_MANY_REQUESTS,[("retry-after","60")],Json(json!({"error":"Authentication temporarily limited"}))).into_response();}
 let bucket=state.auth.csrf(&format!("login:{}",body.username.to_lowercase()));
 let attempts=sqlx::query_scalar::<_,i32>("INSERT INTO auth_login_limits(bucket,attempts) VALUES($1,1) ON CONFLICT(bucket) DO UPDATE SET attempts=CASE WHEN auth_login_limits.window_started<now()-interval '15 minutes' THEN 1 ELSE LEAST(auth_login_limits.attempts+1,100000) END, window_started=CASE WHEN auth_login_limits.window_started<now()-interval '15 minutes' THEN now() ELSE auth_login_limits.window_started END RETURNING attempts").bind(&bucket).fetch_one(&state.db.pool).await;
 let attempts=match attempts{Ok(v)=>v,Err(_)=>return response_error(StatusCode::SERVICE_UNAVAILABLE,"Authentication unavailable")};
 if attempts>5 {
  if attempts==6{tracing::warn!("Repeated authentication failures; account bucket temporarily limited");let _=audit(&state,"anonymous","auth.login.limited",&bucket,"rate_limited").await;}
  return (StatusCode::TOO_MANY_REQUESTS,[("retry-after","900")],Json(json!({"error":"Authentication temporarily limited"}))).into_response();
 }
 let accounts=match crate::auth::configured_accounts(){Ok(a)=>a,Err(_)=>return response_error(StatusCode::SERVICE_UNAVAILABLE,"Authentication unavailable")};
 let account=accounts.as_ref().and_then(|a|a.iter().find(|a|a.username==body.username && !a.disabled));
 let dummy="$argon2id$v=19$m=19456,t=2,p=1$c3ludGhldGljdGVzdHNhbHQ$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
 let hash=account.map(|a|a.password_hash.as_str()).unwrap_or(dummy).to_owned();
 let password=body.password.clone();
 static HASH_SLOTS:tokio::sync::Semaphore=tokio::sync::Semaphore::const_new(2);
 let Ok(permit)=HASH_SLOTS.try_acquire()else{return response_error(StatusCode::TOO_MANY_REQUESTS,"Authentication temporarily limited")};
 let verified=tokio::task::spawn_blocking(move || crate::auth::AuthManager::new("").verify_password(&password,&hash).unwrap_or(false)).await.unwrap_or(false);drop(permit);
 let principal=if let Some(account)=account {if verified{Some((account.role,version(account),account.totp_secret.clone()))}else{None}}
 else if accounts.is_none() && !production(){let user=std::env::var("ADMIN_USERNAME").unwrap_or_else(|_|"admin".into());let password=std::env::var("ADMIN_PASSWORD").unwrap_or_default();if !password.is_empty()&&body.username==user&&equal(&body.password,&password){Some((Role::Admin,digest(&format!("bootstrap:{user}:{password}")),None))}else{None}}else{None};
 let principal=principal.and_then(|(role,version,secret)|{match secret{Some(secret)=>totp_step(&secret,&body.otp,chrono::Utc::now().timestamp()).map(|step|(role,version,Some(step))),None=>Some((role,version,None))}});
 let Some((role,version,step))=principal else{if audit(&state,"anonymous","auth.login.failed",&bucket,"invalid_credentials").await.is_err(){return response_error(StatusCode::SERVICE_UNAVAILABLE,"Authentication unavailable");}return response_error(StatusCode::UNAUTHORIZED,"Invalid credentials")};
 let token=crate::agent_enrollment::random_token();let id=Uuid::new_v4();
 let mut tx=match state.db.pool.begin().await{Ok(t)=>t,Err(_)=>return response_error(StatusCode::SERVICE_UNAVAILABLE,"Authentication unavailable")};
 if let Some(step)=step {
  let consumed=sqlx::query("INSERT INTO auth_mfa_steps(account_version,last_step) VALUES($1,$2) ON CONFLICT(account_version) DO UPDATE SET last_step=excluded.last_step,updated_at=now() WHERE auth_mfa_steps.last_step<excluded.last_step").bind(&version).bind(step).execute(&mut *tx).await;
  if !matches!(consumed,Ok(r) if r.rows_affected()==1){let _=tx.rollback().await;let _=audit(&state,"anonymous","auth.login.failed",&bucket,"invalid_credentials").await;return response_error(StatusCode::UNAUTHORIZED,"Invalid credentials");}
 }
 let role_text=serde_json::to_value(role).expect("role").as_str().unwrap().to_owned();
 if sqlx::query("INSERT INTO human_sessions(session_id,token_hash,username,issued_role,account_version,expires_at) VALUES($1,$2,$3,$4,$5,now()+interval '8 hours')").bind(id).bind(digest(&token)).bind(&body.username).bind(role_text).bind(version).execute(&mut *tx).await.is_err(){return response_error(StatusCode::SERVICE_UNAVAILABLE,"Authentication unavailable");}
 if sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($1,'auth.login.success',$2,$3)").bind(&body.username).bind(id.to_string()).bind("success").execute(&mut *tx).await.is_err(){return response_error(StatusCode::SERVICE_UNAVAILABLE,"Authentication unavailable");}
 if sqlx::query("DELETE FROM auth_login_limits WHERE bucket=$1").bind(bucket).execute(&mut *tx).await.is_err() || tx.commit().await.is_err(){return response_error(StatusCode::SERVICE_UNAVAILABLE,"Authentication unavailable");}
 ([("set-cookie",set_cookie(&token,28800)),("cache-control","no-store".into())],Json(json!({"username":body.username,"role":role,"csrf_token":state.auth.csrf(&token),"expires_in":28800}))).into_response()
}
pub fn router()->Router<Arc<AppState>>{Router::new().route("/login",post(login)).route("/session",get(current)).route("/security",get(security)).route("/logout",post(logout)).route("/sessions",get(list)).route("/sessions/:id",delete(revoke)).route("/revoke-all",post(revoke_all)).route("/users/:username/sessions",delete(revoke_user))}
async fn current(Extension(session):Extension<Session>)->Response{([( "cache-control","no-store")],Json(json!({"username":session.claims.sub,"role":session.claims.role,"csrf_token":session.csrf,"expires_in":session.claims.exp-chrono::Utc::now().timestamp()}))).into_response()}
async fn logout(State(state):State<Arc<AppState>>,Extension(session):Extension<Session>)->Response {
 let result=revoke_atomic(&state,&session.claims.sub,&session.claims.sub,Some(session.id)).await;
 if result.is_err(){return response_error(StatusCode::SERVICE_UNAVAILABLE,"Logout unavailable");}
 ([("set-cookie",set_cookie("",0)),("cache-control","no-store".into())],Json(json!({"success":true}))).into_response()
}
async fn list(State(state):State<Arc<AppState>>,Extension(session):Extension<Session>)->Response{
 match sqlx::query_scalar::<_,serde_json::Value>("SELECT jsonb_build_object('session_id',session_id,'created_at',created_at,'last_seen',last_seen,'expires_at',expires_at,'current',session_id=$2) FROM human_sessions WHERE username=$1 AND revoked_at IS NULL AND expires_at>now() ORDER BY created_at DESC LIMIT 100").bind(&session.claims.sub).bind(session.id).fetch_all(&state.db.pool).await{Ok(v)=>Json(json!({"sessions":v})).into_response(),Err(_)=>response_error(StatusCode::SERVICE_UNAVAILABLE,"Sessions unavailable")}
}
async fn revoke(State(state):State<Arc<AppState>>,Extension(session):Extension<Session>,Path(id):Path<Uuid>)->Response{
 match revoke_atomic(&state,&session.claims.sub,&session.claims.sub,Some(id)).await{Ok(0)=>StatusCode::NOT_FOUND.into_response(),Ok(_)=>StatusCode::NO_CONTENT.into_response(),Err(_)=>response_error(StatusCode::SERVICE_UNAVAILABLE,"Revocation unavailable")}
}
async fn revoke_all(State(state):State<Arc<AppState>>,Extension(session):Extension<Session>)->Response{revoke_for(&state,&session.claims.sub,&session.claims.sub).await}
async fn revoke_user(State(state):State<Arc<AppState>>,Extension(session):Extension<Session>,Path(user):Path<String>)->Response{if session.claims.role!=Role::Admin{return StatusCode::FORBIDDEN.into_response();}revoke_for(&state,&session.claims.sub,&user).await}
async fn revoke_atomic(state:&AppState,actor:&str,user:&str,id:Option<Uuid>)->Result<u64,sqlx::Error>{
 let mut tx=state.db.pool.begin().await?;
 let result=sqlx::query("UPDATE human_sessions SET revoked_at=now() WHERE username=$1 AND ($2::uuid IS NULL OR session_id=$2) AND revoked_at IS NULL").bind(user).bind(id).execute(&mut *tx).await?;
 sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($1,'auth.sessions.revoked',$2,$3)").bind(actor).bind(user).bind(json!({"session_id":id,"count":result.rows_affected(),"result":"revoked"}).to_string()).execute(&mut *tx).await?;
 tx.commit().await?;Ok(result.rows_affected())
}
async fn revoke_for(state:&AppState,actor:&str,user:&str)->Response{if user.len()>100{return StatusCode::BAD_REQUEST.into_response();}match revoke_atomic(state,actor,user,None).await{Ok(_)=>StatusCode::NO_CONTENT.into_response(),Err(_)=>response_error(StatusCode::SERVICE_UNAVAILABLE,"Revocation unavailable")}}
pub fn spawn_cleanup(db:crate::postgres_db::Database){tokio::spawn(async move{let mut interval=tokio::time::interval(std::time::Duration::from_secs(600));loop{interval.tick().await;for query in ["DELETE FROM human_sessions WHERE expires_at<now()-interval '1 day'","DELETE FROM auth_login_limits WHERE window_started<now()-interval '1 day'","DELETE FROM auth_mfa_steps WHERE updated_at<now()-interval '90 days'"]{if sqlx::query(query).execute(&db.pool).await.is_err(){tracing::warn!("Authentication housekeeping unavailable");break;}}}});}
#[cfg(test)]mod tests{use super::*;#[test]fn rfc6238_vector(){let secret="GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";assert_eq!(totp_step(secret,"287082",59),Some(1));assert!(totp_step(secret,"000000",59).is_none());}#[test]fn role_never_promotes(){assert_eq!(minimum(Role::Viewer,Role::Admin),Role::Viewer);assert_eq!(minimum(Role::Admin,Role::Operator),Role::Operator);}}

pub fn api_budget(username:&str)->bool{
 static B:std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String,(std::time::Instant,u32)>>>=std::sync::OnceLock::new();
 let Ok(mut entries)=B.get_or_init(||std::sync::Mutex::new(std::collections::HashMap::new())).lock() else{return false};
 entries.retain(|_,(start,_)|start.elapsed().as_secs()<60);
 if !entries.contains_key(username)&&entries.len()>=1024{return false;}
 let entry=entries.entry(username.into()).or_insert((std::time::Instant::now(),0));if entry.1>=600{return false;}entry.1+=1;true
}
async fn security(State(state):State<Arc<AppState>>,Extension(session):Extension<Session>)->Response{
 if session.claims.role!=Role::Admin{return StatusCode::FORBIDDEN.into_response();}
 let result=sqlx::query_scalar::<_,serde_json::Value>("SELECT jsonb_build_object('failed_logins_15m',(SELECT count(*) FROM audit_events WHERE action='auth.login.failed' AND created_at>now()-interval '15 minutes'),'limited_accounts_24h',(SELECT count(*) FROM audit_events WHERE action='auth.login.limited' AND created_at>now()-interval '24 hours'),'events',COALESCE((SELECT jsonb_agg(to_jsonb(e)) FROM (SELECT actor,action,created_at,details FROM audit_events WHERE action LIKE 'auth.%' ORDER BY id DESC LIMIT 30) e),'[]'::jsonb))").fetch_one(&state.db.pool).await;
 match result{Ok(data)=>Json(data).into_response(),Err(_)=>response_error(StatusCode::SERVICE_UNAVAILABLE,"Authentication audit unavailable")}
}

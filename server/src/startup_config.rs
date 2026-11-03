//! Fail closed before opening listeners or database connections.
use std::{collections::HashMap,io::Read};
const SECRETS:[&str;4]=["JWT_SECRET","AGENT_AUTH_TOKEN","DATABASE_URL","RABBITMQ_URL"];
fn strong(value:&str)->bool {
 let lower=value.to_ascii_lowercase();
 value.len()>=32 && !value.contains(['\r','\n','<','>']) && !["change-me","change_in","change-in","dev-secret","monitor_password","example"].iter().any(|s|lower.contains(s))
}
fn validate(values:&HashMap<String,String>)->Result<bool,String> {
 let mode=values.get("MONITOR_ENV").map(String::as_str).unwrap_or("development");
 if !matches!(mode,"development"|"test"|"production") {return Err("MONITOR_ENV must be development, test or production".into());}
 if mode!="production" {return Ok(false);}
 for key in ["JWT_SECRET","DATABASE_URL","RABBITMQ_URL"] {
  if !values.contains_key(&format!("{key}_FILE")) {return Err(format!("Production requires {key}_FILE"));}
  if values.get(key).is_none_or(|s|s.is_empty()) {return Err(format!("Missing {key}"));}
 }
 if !strong(&values["JWT_SECRET"]) {return Err("JWT_SECRET must be a random secret of at least 32 characters".into());}
 if values.get("AGENT_AUTH_TOKEN").is_some_and(|v|!v.is_empty()) || values.contains_key("AGENT_AUTH_TOKEN_FILE") {return Err("Production forbids shared agent tokens; issue individual device credentials".into());}
 if values.get("ADMIN_PASSWORD").is_some_and(|s|!s.is_empty()) {return Err("Production forbids bootstrap ADMIN_PASSWORD; use managed accounts".into());}
 if values.get("AUTH_USERS_FILE").is_none_or(|s|s.is_empty()) {return Err("Production requires AUTH_USERS_FILE".into());}
 for (key,schemes) in [("DATABASE_URL",&["postgres","postgresql"][..]),("RABBITMQ_URL",&["amqps"][..])] {
  let uri=reqwest::Url::parse(&values[key]).map_err(|_|format!("Invalid {key}"))?;
  if (key=="DATABASE_URL" && uri.username()!="monitor_app") || !schemes.contains(&uri.scheme()) || uri.host_str().is_none() || uri.username().is_empty() || matches!(uri.username(),"guest"|"postgres") || !uri.password().is_some_and(strong) {return Err(format!("{key} requires an explicit non-default account and random password"));}
 }
 Ok(true)
}
pub fn read_secret_file(path:&str,key:&str)->Result<String,Box<dyn std::error::Error>> {
 let mut file=std::fs::File::open(path).map_err(|_|format!("Cannot read {key}_FILE"))?;
 let mut bytes=Vec::new();file.by_ref().take(4097).read_to_end(&mut bytes).map_err(|_|format!("Cannot read {key}_FILE"))?;
 if bytes.len()>4096 {return Err(format!("{key}_FILE exceeds 4096 bytes").into());}
 let value=String::from_utf8(bytes).map_err(|_|format!("{key}_FILE must be UTF-8"))?.trim_end_matches(['\r','\n']).to_owned();
 if value.is_empty() || value.contains(['\r','\n']) {return Err(format!("{key}_FILE must contain one nonempty line").into());}
 Ok(value)
}
pub fn initialize()->Result<bool,Box<dyn std::error::Error>> {
 let mut values:HashMap<String,String>=std::env::vars().collect();
 for key in SECRETS {
  if let Some(path)=values.get(&format!("{key}_FILE")) {
   if values.get(key).is_some_and(|s|!s.is_empty()) {return Err(format!("Configure either {key} or {key}_FILE, not both").into());}
   let value=read_secret_file(path,key)?;
   values.insert(key.into(),value);
  }
 }
 let production=validate(&values)?;
 for key in SECRETS {if let Some(value)=values.get(key){std::env::set_var(key,value);}}
 if production {
  let origin=std::env::var("MONITOR_PUBLIC_ORIGIN").map_err(|_|"Production requires MONITOR_PUBLIC_ORIGIN")?;
  let parsed=reqwest::Url::parse(&origin)?;
  if parsed.scheme()!="https" || parsed.origin().ascii_serialization()!=origin {return Err("MONITOR_PUBLIC_ORIGIN must be an exact HTTPS origin without path".into());}
  let endpoint=reqwest::Url::parse(&std::env::var("AGENT_BROKER_URL").map_err(|_|"Production requires AGENT_BROKER_URL")?)?;
  if endpoint.scheme()!="amqps" || endpoint.host_str().is_none() || !endpoint.username().is_empty() || endpoint.password().is_some(){return Err("AGENT_BROKER_URL must be AMQPS without credentials".into());}
  crate::broker_identity::validate_config().map_err(|_|"Invalid broker administration TLS/secret configuration")?;
  let accounts=crate::auth::configured_accounts()?.ok_or("Managed accounts required")?;
  if !accounts.iter().any(|a|!a.disabled && a.role==crate::auth::Role::Admin) {return Err("Production requires an enabled administrator".into());}
 }
 Ok(production)
}
#[cfg(test)]
mod tests {
 use super::*;
 fn candidate()->HashMap<String,String>{
  let mut v=HashMap::from([("MONITOR_ENV".into(),"production".into()),("AUTH_USERS_FILE".into(),"/run/accounts/users.json".into()),("JWT_SECRET".into(),"a7".repeat(32)),("DATABASE_URL".into(),format!("postgresql://monitor_app:{}@postgres/activity_monitor","c9".repeat(32))),("RABBITMQ_URL".into(),format!("amqps://monitor_server:{}@rabbitmq/%2f","d6".repeat(32)))]);
  for key in ["JWT_SECRET","DATABASE_URL","RABBITMQ_URL"] {v.insert(format!("{key}_FILE"),format!("/run/secrets/{key}"));}v
 }
 #[test] fn production_requires_files_and_rejects_shared_agent_token(){let mut v=candidate();assert_eq!(validate(&v),Ok(true));v.remove("JWT_SECRET_FILE");assert!(validate(&v).is_err());let mut v=candidate();v.insert("AGENT_AUTH_TOKEN".into(),"b8".repeat(32));assert!(validate(&v).is_err());}
 #[test] fn production_rejects_bootstrap_and_defaults(){for (key,value) in [("ADMIN_PASSWORD","admin"),("JWT_SECRET","change-me-in-production"),("DATABASE_URL","postgresql://postgres:postgres@postgres/db"),("RABBITMQ_URL","amqp://guest:guest@rabbitmq/%2f"),("DATABASE_URL","postgresql://monitor_owner:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@postgres/db")] {let mut v=candidate();v.insert(key.into(),value.into());assert!(validate(&v).is_err(),"{key}");}}
 #[test] fn development_unchanged_unknown_mode_rejected(){assert_eq!(validate(&HashMap::new()),Ok(false));let mut v=candidate();v.insert("MONITOR_ENV".into(),"prodution".into());assert!(validate(&v).is_err());}
}

use axum::{extract::{Path,Query,State},http::StatusCode,response::{IntoResponse,Response},Json};
use serde_json::{json,Value};
use std::sync::Arc;
use crate::api::AppState;
#[derive(serde::Deserialize)]
pub struct SessionQuery {day:Option<chrono::NaiveDate>,offset:Option<i64>}
async fn load(state:&AppState,device:uuid::Uuid,query:SessionQuery,limit:i64)->Result<Value,(StatusCode,Json<Value>)> {
 let day=query.day.unwrap_or_else(||chrono::Utc::now().date_naive());let offset=query.offset.unwrap_or(0);
 if offset<0 || offset>1000000 || day.succ_opt().is_none(){return Err((StatusCode::BAD_REQUEST,Json(json!({"error":"Invalid day or offset"}))));}
 let result:Result<Option<Value>,sqlx::Error>=async {
  let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE device_id=$1)").bind(device).fetch_one(&state.db.pool).await?;
  if !exists{return Ok(None);}
  let value=sqlx::query_scalar::<_,Value>(include_str!("session_activity.sql")).bind(device).bind(day).bind(offset).bind(limit).fetch_one(&state.db.pool).await?;
  Ok(Some(value))
 }.await;
 match result {Ok(Some(v))=>Ok(v),Ok(None)=>Err((StatusCode::NOT_FOUND,Json(json!({"error":"Device not found"})))),Err(error)=>{tracing::error!(%error,"Session data unavailable");Err((StatusCode::INTERNAL_SERVER_ERROR,Json(json!({"error":"Session data unavailable"}))))}}
}
pub async fn list(State(state):State<Arc<AppState>>,Path(device):Path<uuid::Uuid>,Query(query):Query<SessionQuery>)->Response {
 match load(&state,device,query,50).await {Ok(value)=>Json(value).into_response(),Err(error)=>error.into_response()}
}
pub async fn export(State(state):State<Arc<AppState>>,Path(device):Path<uuid::Uuid>,Query(mut query):Query<SessionQuery>)->Response {
 query.offset=Some(0);
 let value=match load(&state,device,query,10000).await{Ok(v)=>v,Err(error)=>return error.into_response()};
 if value["total"].as_i64().unwrap_or(0)>10000{return (StatusCode::PAYLOAD_TOO_LARGE,Json(json!({"error":"Too many sessions to export"}))).into_response();}
 let mut csv=String::from("device_id,day_utc,principal,session_id,active_person_seconds,idle_person_seconds,keys_count,mouse_moves_count,clicks_count,available_samples,unavailable_samples,unknown_samples\n");
 for row in value["items"].as_array().into_iter().flatten(){
  let mut fields=vec![device.to_string(),value["day"].as_str().unwrap_or("").into()];
  for key in ["principal","session_id","active_person_seconds","idle_person_seconds","keys_count","mouse_moves_count","clicks_count","available_samples","unavailable_samples","unknown_samples"] {
   let field=if row[key].is_null(){String::new()}else if let Some(text)=row[key].as_str(){text.into()}else{row[key].to_string()};
   fields.push(crate::domains::dashboard::routes::csv_cell(&field));
  }
  csv.push_str(&fields.join(","));csv.push('\n');
 }
 ([("content-type","text/csv; charset=utf-8"),("content-disposition","attachment; filename=monitor-session-activity.csv")],csv).into_response()
}

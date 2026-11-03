use axum::{extract::{State,Path,Query},http::StatusCode,Json};
use serde::Deserialize;
use serde_json::{json,Value};
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;
#[derive(Deserialize)]
pub struct Page { pub limit:Option<i64>, pub offset:Option<i64>, pub q:Option<String> }
async fn load(state:Arc<AppState>,device:Uuid,page:Page,changes:bool)->(StatusCode,Json<Value>) {
    let limit=page.limit.unwrap_or(50).clamp(1,200); let offset=page.offset.unwrap_or(0).clamp(0,1000000);
    let query=page.q.unwrap_or_default();
    if query.len()>256 {return (StatusCode::BAD_REQUEST,Json(json!({"error":"Search too long"})));}
    let result:Result<Value,sqlx::Error>=async {
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM devices WHERE device_id=$1)").bind(device).fetch_one(&state.db.pool).await?;
        if !exists {return Ok(Value::Null);}
        // Literal, case-insensitive substring search, not SQL wildcard syntax.
        let (sql,count_sql)=if changes {(
            "SELECT to_jsonb(x) FROM (SELECT id,scope,source_key,action,before_value,after_value,timestamp FROM inventory_changes WHERE device_id=$1 AND ($2='' OR strpos(lower(COALESCE(after_value->>'app_name',before_value->>'app_name','')),lower($2))>0) ORDER BY timestamp DESC,id DESC LIMIT $3 OFFSET $4) x",
            "SELECT count(*) FROM inventory_changes WHERE device_id=$1 AND ($2='' OR strpos(lower(COALESCE(after_value->>'app_name',before_value->>'app_name','')),lower($2))>0)"
        )}else{(
            "SELECT to_jsonb(x) FROM (SELECT scope,source_key,app_name,version,changed_at FROM inventory_current WHERE device_id=$1 AND ($2='' OR strpos(lower(app_name),lower($2))>0) ORDER BY app_name,scope,source_key LIMIT $3 OFFSET $4) x",
            "SELECT count(*) FROM inventory_current WHERE device_id=$1 AND ($2='' OR strpos(lower(app_name),lower($2))>0)"
        )};
        let items=sqlx::query_scalar::<_,Value>(sql).bind(device).bind(&query).bind(limit).bind(offset).fetch_all(&state.db.pool).await?;
        let total:i64=sqlx::query_scalar(count_sql).bind(device).bind(&query).fetch_one(&state.db.pool).await?;
        let sources=sqlx::query_scalar::<_,Value>("SELECT jsonb_build_object('scope',scope,'observed_at',observed_at,'last_complete_at',last_complete_at,'last_error',last_error,'item_count',item_count,'stale',last_complete_at IS NULL OR last_complete_at < now()-(expected_interval_days+1)*interval '1 day') FROM inventory_scan_state WHERE device_id=$1 ORDER BY scope")
            .bind(device).fetch_all(&state.db.pool).await?;
        Ok(json!({"items":items,"sources":sources,"total":total,"limit":limit,"offset":offset}))
    }.await;
    match result {
        Ok(Value::Null)=>(StatusCode::NOT_FOUND,Json(json!({"error":"Device not found"}))),
        Ok(data)=>(StatusCode::OK,Json(data)),
        Err(error)=>{tracing::error!(%error,"Inventory state query failed");(StatusCode::INTERNAL_SERVER_ERROR,Json(json!({"error":"Inventory unavailable"})))}
    }
}
pub async fn current(State(state):State<Arc<AppState>>,Path(device):Path<Uuid>,Query(page):Query<Page>)->(StatusCode,Json<Value>) {load(state,device,page,false).await}
pub async fn changes(State(state):State<Arc<AppState>>,Path(device):Path<Uuid>,Query(page):Query<Page>)->(StatusCode,Json<Value>) {load(state,device,page,true).await}

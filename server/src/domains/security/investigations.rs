use axum::{extract::{Path, Query, State}, http::{HeaderMap, StatusCode}, Json};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;

fn actor(state: &AppState, headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|h|h.to_str().ok()).and_then(|h|h.strip_prefix("Bearer "))
        .and_then(|token|state.auth.verify_token(token).ok()).map(|claims|claims.sub)
}
type Response = (StatusCode, Json<Value>);
fn failure(error: sqlx::Error) -> Response {
    tracing::error!(%error, "Security investigation operation failed");
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Investigation operation failed"})))
}
#[derive(Deserialize)]
pub struct List { device_id: Option<Uuid>, limit: Option<i64>, offset: Option<i64> }
pub async fn list(State(state): State<Arc<AppState>>, Query(q): Query<List>) -> Response {
    let limit = q.limit.unwrap_or(50).clamp(1,200);
    let offset = q.offset.unwrap_or(0).clamp(0,1000000);
    match sqlx::query_scalar::<_,Value>("SELECT to_jsonb(i) FROM security_investigations i WHERE ($1::uuid IS NULL OR device_id=$1) ORDER BY created_at DESC,id LIMIT $2 OFFSET $3")
        .bind(q.device_id).bind(limit + 1).bind(offset).fetch_all(&state.db.pool).await {
        Ok(mut items) => { let more=items.len()>limit as usize; items.truncate(limit as usize);
            (StatusCode::OK,Json(json!({"items":items,"has_more":more,"limit":limit,"offset":offset}))) },
        Err(e)=>failure(e)
    }
}
#[derive(Deserialize)]
pub struct Create { device_id: Uuid, title: String, reason: String, event_from: DateTime<Utc>, event_to: DateTime<Utc> }
pub async fn create(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(q): Json<Create>) -> Response {
    let Some(actor)=actor(&state,&headers) else {return (StatusCode::UNAUTHORIZED,Json(json!({"error":"Unauthorized"})));};
    if q.title.trim().is_empty() || q.title.len()>200 || q.reason.trim().is_empty() || q.reason.len()>4000 || q.event_to<q.event_from {
        return (StatusCode::BAD_REQUEST,Json(json!({"error":"Title, reason and ordered UTC interval are required"})));
    }
    let result: Result<Option<Value>,sqlx::Error> = async {
        let mut tx=state.db.pool.begin().await?;
        sqlx::query("SET LOCAL lock_timeout = '5s'").execute(&mut *tx).await?;
        // Serialize with retention: once creation commits, no subsequent batch can remove held events.
        sqlx::query("SELECT pg_advisory_xact_lock(734203401)").execute(&mut *tx).await?;
        let row=sqlx::query_scalar::<_,Value>("INSERT INTO security_investigations(id,device_id,title,reason,event_from,event_to,created_by) SELECT $1,$2,$3,$4,$5,$6,$7 WHERE EXISTS(SELECT 1 FROM devices WHERE device_id=$2) RETURNING to_jsonb(security_investigations)")
            .bind(Uuid::new_v4()).bind(q.device_id).bind(q.title.trim()).bind(q.reason.trim()).bind(q.event_from).bind(q.event_to).bind(actor).fetch_optional(&mut *tx).await?;
        tx.commit().await?; Ok(row)
    }.await;
    match result {
        Ok(Some(row))=>(StatusCode::CREATED,Json(row)),
        Ok(None)=>(StatusCode::NOT_FOUND,Json(json!({"error":"Device not found"}))),
        Err(e)=>failure(e)
    }
}
#[derive(Deserialize)]
pub struct Release { reason: String }
pub async fn release(State(state): State<Arc<AppState>>, Path(id): Path<Uuid>, headers: HeaderMap, Json(q): Json<Release>) -> Response {
    let Some(actor)=actor(&state,&headers) else {return (StatusCode::UNAUTHORIZED,Json(json!({"error":"Unauthorized"})));};
    if q.reason.trim().is_empty() || q.reason.len()>4000 {
        return (StatusCode::BAD_REQUEST,Json(json!({"error":"Release reason is required (maximum 4000 bytes)"})));
    }
    match sqlx::query_scalar::<_,Value>("UPDATE security_investigations SET released_at=now(),release_reason=$2,released_by=$3 WHERE id=$1 AND released_at IS NULL RETURNING to_jsonb(security_investigations)")
        .bind(id).bind(q.reason.trim()).bind(actor).fetch_optional(&state.db.pool).await {
        Ok(Some(row))=>(StatusCode::OK,Json(row)),
        Ok(None)=>(StatusCode::CONFLICT,Json(json!({"error":"Investigation missing or already released"}))),
        Err(e)=>failure(e)
    }
}

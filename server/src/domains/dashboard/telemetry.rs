use axum::{extract::{State, Path}, http::StatusCode, Json};
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;

pub async fn get_state(State(state): State<Arc<AppState>>, Path(device): Path<Uuid>)
    -> (StatusCode, Json<serde_json::Value>) {
    let result = sqlx::query_scalar::<_, serde_json::Value>(r#"
        SELECT jsonb_build_object('device_id',d.device_id,'queried_at',now(),
            'resources',(SELECT jsonb_build_object('observed_at',s.observed_at,'received_at',s.received_at,
                'stale',s.observed_at < now()-interval '180 seconds','data',s.payload)
                FROM device_telemetry_state s WHERE s.device_id=d.device_id AND s.kind='resources'),
            'health',(SELECT jsonb_build_object('observed_at',s.observed_at,'received_at',s.received_at,
                'stale',s.observed_at < now()-interval '900 seconds','data',s.payload)
                FROM device_telemetry_state s WHERE s.device_id=d.device_id AND s.kind='health'))
        FROM devices d WHERE d.device_id=$1
    "#).bind(device).fetch_optional(&state.db.pool).await;
    match result {
        Ok(Some(value)) => (StatusCode::OK, Json(value)),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"Device not found"}))),
        Err(error) => {
            tracing::error!(%error, "Cannot read device telemetry state");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"Telemetry unavailable"})))
        }
    }
}

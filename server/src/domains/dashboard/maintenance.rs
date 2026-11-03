use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;
use crate::api::AppState;

pub async fn get_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query_scalar::<_, serde_json::Value>(r#"
        WITH candidates AS (
            SELECT c.oid,n.nspname AS schema_name,c.relname AS relation_name,c.reltuples
            FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
            WHERE n.nspname IN ('public','_timescaledb_internal') AND c.relkind IN ('r','m','p')
            ORDER BY n.nspname,c.relname LIMIT 201
        ), sizes AS (
            SELECT schema_name,relation_name,pg_total_relation_size(oid) AS total_bytes,
                pg_relation_size(oid) AS heap_bytes,pg_indexes_size(oid) AS index_bytes,
                CASE WHEN reltuples>=0 THEN reltuples::bigint ELSE NULL END AS estimated_rows
            FROM candidates ORDER BY schema_name,relation_name LIMIT 200
        )
        SELECT jsonb_build_object(
            'observed_at', now(),
            'worker_settings',COALESCE((SELECT jsonb_object_agg(name,setting) FROM pg_settings WHERE name IN ('max_worker_processes','max_parallel_workers','max_logical_replication_workers','timescaledb.max_background_workers')),'{}'::jsonb),
            'database_size_bytes', pg_database_size(current_database()),
            'relations',COALESCE((SELECT jsonb_agg(to_jsonb(s) ORDER BY schema_name,relation_name) FROM sizes s),'[]'::jsonb),
            'relations_truncated',(SELECT count(*)>200 FROM candidates),
            'relation_limit',200,
            'metrics', (SELECT to_jsonb(m) FROM public.maintenance_status m WHERE job = 'metrics-retention'),
            'history', COALESCE((SELECT jsonb_agg(to_jsonb(h) ORDER BY job)
                FROM public.maintenance_job_status h), '[]'::jsonb))
    "#).fetch_one(&state.db.pool).await;
    match result {
        Ok(value) => (StatusCode::OK, Json(value)),
        Err(error) => {
            tracing::error!(%error, "Cannot read maintenance status");
            (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Maintenance status unavailable"})))
        }
    }
}

#[derive(serde::Deserialize)]
pub struct StorageQuery { mode: Option<String>, offset: Option<i64> }
pub async fn storage(State(state): State<Arc<AppState>>, axum::extract::Query(q): axum::extract::Query<StorageQuery>) -> impl IntoResponse {
    let mode=q.mode.as_deref().unwrap_or("relations");
    if !matches!(mode,"relations"|"hypertables") {
        return (StatusCode::BAD_REQUEST,Json(serde_json::json!({"error":"Invalid storage mode"})));
    }
    let offset=q.offset.unwrap_or(0).clamp(0,1000000);
    let sql=if mode=="hypertables" {r#"
        SELECT jsonb_build_object('schema_name',h.hypertable_schema,'relation_name',h.hypertable_name,
            'total_bytes',COALESCE(s.total_bytes,0),'heap_bytes',COALESCE(s.table_bytes,0),
            'index_bytes',COALESCE(s.index_bytes,0),'toast_bytes',COALESCE(s.toast_bytes,0),'estimated_rows',NULL)
        FROM (SELECT hypertable_schema,hypertable_name FROM timescaledb_information.hypertables
              ORDER BY hypertable_schema,hypertable_name LIMIT 51 OFFSET $1) h
        LEFT JOIN LATERAL (SELECT sum(total_bytes)::bigint AS total_bytes,sum(table_bytes)::bigint AS table_bytes,sum(index_bytes)::bigint AS index_bytes,sum(toast_bytes)::bigint AS toast_bytes FROM hypertable_detailed_size(format('%I.%I',h.hypertable_schema,h.hypertable_name)::regclass)) s ON true
        ORDER BY h.hypertable_schema,h.hypertable_name
    "#} else {r#"
        SELECT jsonb_build_object('schema_name',n.nspname,'relation_name',c.relname,
            'total_bytes',pg_total_relation_size(c.oid),'heap_bytes',pg_relation_size(c.oid),
            'index_bytes',pg_indexes_size(c.oid),'estimated_rows',CASE WHEN c.reltuples>=0 THEN c.reltuples::bigint ELSE NULL END)
        FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
        WHERE n.nspname IN ('public','_timescaledb_internal') AND c.relkind IN ('r','m','p')
        ORDER BY n.nspname,c.relname LIMIT 51 OFFSET $1
    "#};
    match sqlx::query_scalar::<_,serde_json::Value>(sql).bind(offset).fetch_all(&state.db.pool).await {
        Ok(mut items)=>{let more=items.len()>50;items.truncate(50);(StatusCode::OK,Json(serde_json::json!({"items":items,"has_more":more,"offset":offset,"limit":50,"mode":mode,"observed_at":chrono::Utc::now()})))},
        Err(error)=>{tracing::warn!(%error,"Storage inspection unavailable");(StatusCode::SERVICE_UNAVAILABLE,Json(serde_json::json!({"error":"Storage inspection unavailable"})))}
    }
}

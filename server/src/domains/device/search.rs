use axum::{extract::{Query,State},http::StatusCode,Json};
use std::sync::Arc;
use serde::Deserialize;
use serde_json::{json,Value};
use crate::api::AppState;
#[derive(Deserialize)]
pub struct Search { q:Option<String>, status:Option<String>, group:Option<String>, sort:Option<String>, page:Option<i64> }
pub async fn search(State(state):State<Arc<AppState>>,Query(q):Query<Search>)->(StatusCode,Json<Value>) {
    let query=q.q.unwrap_or_default(); let status=q.status.unwrap_or_else(||"all".into());let group=q.group.unwrap_or_default();let sort=q.sort.unwrap_or_else(||"name".into());let page=q.page.unwrap_or(1).clamp(1,50001);
    if query.len()>256 || group.len()>100 || !matches!(status.as_str(),"all"|"online"|"offline"|"stale") || !matches!(sort.as_str(),"name"|"recent") {return (StatusCode::BAD_REQUEST,Json(json!({"error":"Invalid equipment filters"})));}
    let result=sqlx::query_scalar::<_,Value>(r#"
        WITH base AS (
            SELECT d.*,CASE WHEN last_seen<=now()-$1::bigint*interval '1 second' THEN 'offline'
                WHEN last_seen<=now()-$2::bigint*interval '1 second' THEN 'stale' ELSE 'online' END AS connection_status
            FROM devices d
        ), filtered AS (
            SELECT * FROM base WHERE ($3='' OR strpos(lower(concat_ws(' ',hostname,nickname,mac_address,device_id::text)),lower($3))>0)
              AND ($4='all' OR connection_status=$4) AND ($5='' OR group_name=$5)
        ), selected AS (
            SELECT * FROM filtered ORDER BY CASE WHEN $6='name' THEN lower(COALESCE(NULLIF(nickname,''),hostname)) END,
              CASE WHEN $6='recent' THEN last_seen END DESC,device_id LIMIT 20 OFFSET $7
        ), rows AS (
            SELECT to_jsonb(d)||jsonb_build_object('id',d.device_id,'online',d.connection_status<>'offline','stale',d.connection_status='stale',
                'status',d.connection_status,'agent_health',CASE WHEN h.device_id IS NULL THEN NULL ELSE jsonb_build_object('status',h.payload->'status',
                'monitoring_paused',h.payload->'monitoring_paused','observed_at',h.observed_at,'stale',h.observed_at<now()-interval '900 seconds') END) AS item,
                row_number() OVER(ORDER BY CASE WHEN $6='name' THEN lower(COALESCE(NULLIF(d.nickname,''),d.hostname)) END,CASE WHEN $6='recent' THEN d.last_seen END DESC,d.device_id) AS n
            FROM selected d LEFT JOIN device_telemetry_state h ON h.device_id=d.device_id AND h.kind='health'
        ) SELECT jsonb_build_object('devices',COALESCE((SELECT jsonb_agg(item ORDER BY n) FROM rows),'[]'::jsonb),
            'total',(SELECT count(*) FROM filtered),'page',$8::bigint,'page_size',20,
            'counts',(SELECT jsonb_build_object('total',count(*),'online',count(*) FILTER(WHERE connection_status='online'),'offline',count(*) FILTER(WHERE connection_status='offline'),'stale',count(*) FILTER(WHERE connection_status='stale')) FROM base),
            'groups',COALESCE((SELECT jsonb_agg(group_name ORDER BY group_name) FROM (SELECT DISTINCT group_name FROM base WHERE group_name<>'') g),'[]'::jsonb))
    "#).bind(state.config.online_threshold_seconds.max(1)).bind(state.config.stale_threshold_seconds.max(1).min(state.config.online_threshold_seconds.max(1)))
        .bind(query.trim()).bind(status).bind(group).bind(sort).bind((page-1)*20).bind(page).fetch_one(&state.db.pool).await;
    match result {Ok(data)=>(StatusCode::OK,Json(data)),Err(error)=>{tracing::error!(%error,"Equipment search failed");(StatusCode::INTERNAL_SERVER_ERROR,Json(json!({"error":"Equipment search unavailable"})))}}
}

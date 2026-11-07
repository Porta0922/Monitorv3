use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode}, Json};
use lapin::{Connection, ConnectionProperties, options::*, types::AMQPValue};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use crate::api::AppState;
const QUEUES: [&str;7] = ["activity_queue","inventory_queue","heartbeat_queue","usb_queue","wifi_queue","running_apps_queue","security_queue"];
type Response = (StatusCode, Json<Value>);
type Error = Box<dyn std::error::Error + Send + Sync>;
async fn connect(state: &AppState) -> Result<Connection, Error> {
    Ok(tokio::time::timeout(Duration::from_secs(5),Connection::connect(&state.rabbitmq_url,ConnectionProperties::default())).await??)
}
pub async fn status(State(state): State<Arc<AppState>>) -> Response {
    let conn=match connect(&state).await {Ok(c)=>c,Err(e)=>{tracing::warn!(%e,"DLQ connection failed");return (StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"Broker unavailable"})));}};
    let result=tokio::time::timeout(Duration::from_secs(8),async {
        let mut rows=Vec::new();
        for name in QUEUES {
            let channel=conn.create_channel().await?;
            match channel.queue_declare(format!("{name}.dead").into(),QueueDeclareOptions{passive:true,..Default::default()},Default::default()).await {
                Ok(q)=>rows.push(json!({"queue":name,"ready":q.message_count(),"consumers":q.consumer_count(),"available":true})),
                Err(_)=>rows.push(json!({"queue":name,"ready":null,"available":false}))
            }
            let _=channel.close(200,"Status read complete".into()).await;
        }
        Ok::<_,Error>(rows)
    }).await;
    let _=tokio::time::timeout(Duration::from_secs(1),conn.close(200,"Request complete".into())).await;
    match result {Ok(Ok(rows))=>(StatusCode::OK,Json(json!({"queues":rows,"observed_at":chrono::Utc::now(),"replay_days":30}))),
        _=>(StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"Cannot read dead-letter queues"})))}
}
#[derive(Deserialize)]
pub struct Replay { limit: u16, reason: String }
pub async fn replay(State(state): State<Arc<AppState>>,Path(queue):Path<String>,headers:HeaderMap,Json(body):Json<Replay>) -> Response {
    if !QUEUES.contains(&queue.as_str()) || body.limit==0 || body.limit>20 || body.reason.trim().is_empty() || body.reason.len()>2000 {
        return (StatusCode::BAD_REQUEST,Json(json!({"error":"Known queue, limit 1..20 and reason required"})));
    }
    let actor=headers.get("authorization").and_then(|v|v.to_str().ok()).and_then(|v|v.strip_prefix("Bearer "))
        .and_then(|token|state.auth.verify_token(token).ok()).map(|c|c.sub);
    let Some(actor)=actor else {return (StatusCode::UNAUTHORIZED,Json(json!({"error":"Unauthorized"})));};
    let mut lock=match state.db.pool.begin().await {Ok(t)=>t,Err(_)=>return (StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"Database unavailable"})))};
    let acquired=sqlx::query_scalar::<_,bool>("SELECT pg_try_advisory_xact_lock(734203402)").fetch_one(&mut *lock).await.unwrap_or(false);
    if !acquired {return (StatusCode::CONFLICT,Json(json!({"error":"A replay is already running"})));}
    let operation=uuid::Uuid::new_v4();
    // Persist intent before touching the broker, independently of the concurrency lock.
    let audit=sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($1,'dlq.replay.started',$2,$3)")
        .bind(&actor).bind(&queue).bind(json!({"operation":operation,"limit":body.limit,"reason":body.reason}).to_string()).execute(&state.db.pool).await;
    if audit.is_err() {return (StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"Cannot record replay intent"})));}
    let mut moved=0u16;
    let result=async {
        let conn=connect(&state).await?;
        let result=tokio::time::timeout(Duration::from_secs(15),async {
            let channel=conn.create_channel().await?;
            channel.confirm_select(Default::default()).await?;
            // Never silently create a destination that has lost its consumer configuration.
            channel.queue_declare(queue.as_str().into(),QueueDeclareOptions{passive:true,..Default::default()},Default::default()).await?;
            for _ in 0..body.limit {
                let Some(message)=channel.basic_get(format!("{queue}.dead").into(),BasicGetOptions{no_ack:false}).await? else {break;};
                let delivery=&message.delivery;
                let event=serde_json::from_slice::<Value>(&delivery.data);
                let eligible=event.as_ref().ok().is_some_and(|e| crate::replay_policy::validate(e).is_ok()
                    && e["device_id"].as_str().and_then(|id|uuid::Uuid::parse_str(id).ok()).is_some());
                let authorized=match event.as_ref(){Ok(event)=>crate::broker_identity::validate_delivery(&state.db,&delivery,event).await?,Err(_)=>false};
                if !eligible || !authorized {
                    delivery.nack(BasicNackOptions{multiple:false,requeue:true}).await?;
                    return Ok::<_,Error>(Some("Head message has invalid/revoked identity or date, or exceeds replay window; retained in DLQ"));
                }
                let mut headers=delivery.properties.headers().clone().unwrap_or_default();
                headers.insert("x-monitor-attempts".into(),AMQPValue::LongUInt(0));
                let confirm=channel.basic_publish("".into(),queue.as_str().into(),BasicPublishOptions{mandatory:true,..Default::default()},&delivery.data,
                    crate::broker_identity::retry_properties(&delivery,headers)).await?.await?;
                if !matches!(confirm,lapin::Confirmation::Ack(None)) {return Err("Replay publication not confirmed".into());}
                if !delivery.ack(Default::default()).await? { return Err("Delivery acknowledgement was not sent".into()); }
                moved+=1;
            }
            Ok(None)
        }).await;
        // Closing requeues any original for which publish/ack completion is uncertain.
        let _=tokio::time::timeout(Duration::from_secs(1),conn.close(200,"Replay complete".into())).await;
        result?
    }.await;
    let (status,detail)=match result {
        Ok(blocked)=>(StatusCode::OK,json!({"blocked":blocked,"uncertain":false})),
        Err(e)=>{tracing::warn!(%e,%operation,"Replay incomplete");(StatusCode::SERVICE_UNAVAILABLE,json!({"error":"Replay interrupted; refresh before retrying","uncertain":true}))}
    };
    let recorded=sqlx::query("INSERT INTO audit_events(actor,action,target,details) VALUES($1,'dlq.replay.finished',$2,$3)")
        .bind(actor).bind(queue).bind(json!({"operation":operation,"acknowledged":moved,"result":detail}).to_string()).execute(&state.db.pool).await.is_ok();
    let _=lock.rollback().await;
    (status,Json(json!({"operation":operation,"acknowledged":moved,"result":detail,"audit_completion_recorded":recorded})))
}

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};
use chrono::{NaiveDate, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct DailyQuery { pub from: Option<NaiveDate>, pub to: Option<NaiveDate> }

#[derive(Serialize, sqlx::FromRow)]
pub struct DailyMetrics {
    pub day: NaiveDate,
    pub resource_samples: i64,
    pub input_samples: i64,
    pub input_available_samples:i64,
    pub input_unavailable_samples:i64,
    pub input_unknown_samples:i64,
    pub cpu_average: Option<f64>,
    pub cpu_peak: Option<f64>,
    pub memory_average: Option<f64>,
    pub memory_peak: Option<f64>,
    pub memory_used_mb_average: Option<f64>,
    pub active_seconds: Option<i64>,
    pub idle_seconds: Option<i64>,
    pub keys_count: Option<i64>,
    pub mouse_moves_count: Option<i64>,
    pub clicks_count: Option<i64>,
    pub source: String,
}

impl DailyQuery {
    fn bounds(&self, today: NaiveDate) -> Result<(NaiveDate, NaiveDate), &'static str> {
        let to = self.to.unwrap_or(today);
        let from = match self.from {
            Some(from) => from,
            None => to.checked_sub_signed(Duration::days(29)).ok_or("Date range is out of bounds")?,
        };
        if to > today || from > to || (to-from).num_days() >= 366 {
            return Err("Use an ordered UTC date range of at most 366 days, ending today or earlier");
        }
        Ok((from, to))
    }
}

pub async fn get_daily_metrics(
    State(state): State<Arc<AppState>>, Path(device_id): Path<Uuid>, Query(query): Query<DailyQuery>,
) -> impl IntoResponse {
    let (from,to) = match query.bounds(Utc::now().date_naive()) {
        Ok(bounds) => bounds,
        Err(message) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":message}))),
    };
    match state.db.get_device_by_id(device_id).await {
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"device not found"}))),
        Err(error) => {
            tracing::error!("Daily metrics device lookup failed: {}", error);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"daily metrics unavailable"})));
        }
        Ok(Some(_)) => {}
    }
    match sqlx::query_as::<_, DailyMetrics>(include_str!("daily_metrics.sql"))
        .bind(device_id).bind(from).bind(to).fetch_all(&state.db.pool).await {
        Ok(days) => (StatusCode::OK, Json(serde_json::json!({
            "device_id":device_id,"from":from,"to":to,"timezone":"UTC",
            "average_basis":"sample","days":days
        }))),
        Err(error) => {
            tracing::error!("Daily metrics query failed: {}", error);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"daily metrics unavailable"})))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daily_range_is_bounded_and_never_reversed_or_future() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        assert_eq!(DailyQuery {from:None,to:None}.bounds(today).unwrap(), (today-Duration::days(29),today));
        assert!(DailyQuery {from:Some(today-Duration::days(365)),to:None}.bounds(today).is_ok());
        for query in [DailyQuery {from:Some(today-Duration::days(366)),to:None},
                      DailyQuery {from:Some(today),to:Some(today-Duration::days(1))},
                      DailyQuery {from:None,to:Some(today+Duration::days(1))},
                      DailyQuery {from:None,to:Some(NaiveDate::MIN)}] {
            assert!(query.bounds(today).is_err());
        }
    }
}

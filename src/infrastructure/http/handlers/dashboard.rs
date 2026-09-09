//! `GET /api/v1/dashboard` — aggregates for the admin home (session + admin role).

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::{Json, extract::State};
use chrono::{Duration, Utc};
use serde_json::{Value, json};

use crate::{
    domain::processing::entities::{Job, JobStatus},
    errors::AppResult,
    infrastructure::http::middleware::session::AdminUser,
    state::AppState,
};

fn tally<'a>(iter: impl Iterator<Item = &'a str>) -> BTreeMap<String, u64> {
    let mut map = BTreeMap::new();
    for k in iter {
        *map.entry(k.to_string()).or_insert(0) += 1;
    }
    map
}

fn percentile(sorted: &[i64], p: f64) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    Some(sorted[idx.min(sorted.len() - 1)])
}

/// GET /api/v1/dashboard
pub async fn overview(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Value>> {
    let jobs = state.processing.jobs.list().await?;
    let since = Utc::now() - Duration::hours(24);

    let terminal: Vec<&Job> = jobs
        .iter()
        .filter(|j| matches!(j.status, JobStatus::Completed | JobStatus::Failed))
        .collect();

    let mut durations: Vec<i64> = terminal.iter().filter_map(|j| j.duration_ms()).collect();
    durations.sort_unstable();
    let avg_ms = if durations.is_empty() {
        None
    } else {
        Some(durations.iter().sum::<i64>() / durations.len() as i64)
    };

    let processings = json!({
        "total": jobs.len(),
        "last_24h": jobs.iter().filter(|j| j.created_at >= since).count(),
        "total_rows": jobs.iter().map(Job::total_rows).sum::<u64>(),
        "by_kind": tally(jobs.iter().map(|j| j.kind.as_str())),
        "by_origin": tally(jobs.iter().map(|j| j.origin.as_str())),
        "by_status": tally(jobs.iter().map(|j| j.status.as_str())),
        "avg_ms": avg_ms,
        "p95_ms": percentile(&durations, 0.95),
    });

    let in_flight = json!({
        "pending": jobs.iter().filter(|j| j.status == JobStatus::Pending).count(),
        "running": jobs.iter().filter(|j| j.status == JobStatus::Running).count(),
        "failed": jobs.iter().filter(|j| j.status == JobStatus::Failed).count(),
    });

    let counts = json!({
        "users": state.auth.users.list().await?.len(),
        "roles": state.roles.list().await?.len(),
        "api_clients": state.api_clients.list().await?.iter().filter(|c| c.active).count(),
    });

    let recent: Vec<Value> = jobs.iter().take(8).map(Job::to_summary_json).collect();

    Ok(Json(json!({
        "processings": processings,
        "in_flight": in_flight,
        "counts": counts,
        "recent": recent,
    })))
}

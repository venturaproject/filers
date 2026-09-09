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

    let failed = terminal
        .iter()
        .filter(|j| j.status == JobStatus::Failed)
        .count();
    let error_rate = if terminal.is_empty() {
        None
    } else {
        Some(failed as f64 / terminal.len() as f64)
    };

    // Per-operation rollup: count / failed / avg / p95.
    let mut ops: BTreeMap<&str, (u64, u64, Vec<i64>)> = BTreeMap::new();
    for j in &jobs {
        let e = ops.entry(j.operation.as_str()).or_default();
        e.0 += 1;
        if j.status == JobStatus::Failed {
            e.1 += 1;
        }
        if let Some(d) = j.duration_ms() {
            e.2.push(d);
        }
    }
    let per_operation: Value = ops
        .into_iter()
        .map(|(op, (count, failed, mut ds))| {
            ds.sort_unstable();
            let avg = (!ds.is_empty()).then(|| ds.iter().sum::<i64>() / ds.len() as i64);
            (
                op.to_string(),
                json!({
                    "count": count,
                    "failed": failed,
                    "avg_ms": avg,
                    "p95_ms": percentile(&ds, 0.95),
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into();

    // Hourly activity for the last 24h.
    let now = Utc::now();
    let mut timeline: Vec<Value> = Vec::with_capacity(24);
    for h in (0..24).rev() {
        let start = now - Duration::hours(h + 1);
        let end = now - Duration::hours(h);
        let in_bucket: Vec<&Job> = jobs
            .iter()
            .filter(|j| j.created_at >= start && j.created_at < end)
            .collect();
        timeline.push(json!({
            "hour": end.format("%Y-%m-%dT%H:00:00Z").to_string(),
            "total": in_bucket.len(),
            "failed": in_bucket.iter().filter(|j| j.status == JobStatus::Failed).count(),
        }));
    }

    let processings = json!({
        "total": jobs.len(),
        "last_24h": jobs.iter().filter(|j| j.created_at >= since).count(),
        "total_rows": jobs.iter().map(Job::total_rows).sum::<u64>(),
        "by_kind": tally(jobs.iter().map(|j| j.kind.as_str())),
        "by_origin": tally(jobs.iter().map(|j| j.origin.as_str())),
        "by_status": tally(jobs.iter().map(|j| j.status.as_str())),
        "by_operation": tally(jobs.iter().map(|j| j.operation.as_str())),
        "avg_ms": avg_ms,
        "p95_ms": percentile(&durations, 0.95),
        "error_rate": error_rate,
        "per_operation": per_operation,
        "timeline": timeline,
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

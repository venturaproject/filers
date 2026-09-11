//! Query parsing + envelope for the admin list endpoints, matching the shape
//! the frontend's `normalizePaginatedPayload` expects.

use std::collections::HashMap;

use serde_json::{Value, json};

pub struct PageParams {
    pub page: usize,
    pub per_page: usize,
    pub search: Option<String>,
}

impl PageParams {
    pub fn from_query(q: &HashMap<String, String>) -> Self {
        let page = q
            .get("page")
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|p| *p >= 1)
            .unwrap_or(1);
        let per_page = q
            .get("per_page")
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|p| *p >= 1)
            .unwrap_or(20)
            .min(1000);
        let search = q
            .get("search")
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
        Self {
            page,
            per_page,
            search,
        }
    }
}

/// Paginate `rows` (already filtered + serialised) into the list envelope.
pub fn envelope(rows: Vec<Value>, params: &PageParams) -> Value {
    let total = rows.len();
    let last_page = total.div_ceil(params.per_page).max(1);
    let start = (params.page - 1) * params.per_page;
    let data: Vec<Value> = rows.into_iter().skip(start).take(params.per_page).collect();

    json!({
        "data": data,
        "current_page": params.page,
        "last_page": last_page,
        "per_page": params.per_page,
        "total": total,
    })
}

//! Descriptive role/permission catalogue for the admin UI.
//!
//! These are *management* records surfaced by `/api/v1/roles` and
//! `/api/v1/permissions`. Actual API authorisation still runs off
//! [`crate::domain::auth::entities::Role`]; this module is not an enforcement
//! point.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct Permission {
    pub id: u32,
    pub name: String,
    pub guard_name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Permission {
    pub fn to_json(&self) -> Value {
        let created = self.created_at.to_rfc3339();
        json!({
            "id": self.id,
            "name": self.name,
            "guard_name": self.guard_name,
            "description": self.description,
            "created_at": created,
            "createdAt": created,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Role {
    pub id: u32,
    pub name: String,
    pub guard_name: String,
    pub description: Option<String>,
    pub permission_ids: Vec<u32>,
    pub created_at: DateTime<Utc>,
}

impl Role {
    /// `permissions` is the resolved list of this role's [`Permission`]s;
    /// `users_count` is how many users currently hold the role.
    pub fn to_json(&self, permissions: &[Permission], users_count: usize) -> Value {
        let created = self.created_at.to_rfc3339();
        let perms: Vec<Value> = permissions.iter().map(Permission::to_json).collect();
        json!({
            "id": self.id,
            "name": self.name,
            "guard_name": self.guard_name,
            "description": self.description,
            "permissions": perms,
            "permission_ids": self.permission_ids,
            "permissions_count": permissions.len(),
            "users_count": users_count,
            "created_at": created,
            "createdAt": created,
        })
    }
}

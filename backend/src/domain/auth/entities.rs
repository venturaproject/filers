use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::User => "user",
        }
    }

    /// Admins implicitly hold every permission.
    pub fn is_full_access(&self) -> bool {
        matches!(self, Role::Admin)
    }

    /// Derive the primary security role from a set of assigned role names.
    /// Anything containing `admin` grants admin; otherwise it's a plain user.
    pub fn from_role_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if names
            .into_iter()
            .any(|n| n.as_ref().eq_ignore_ascii_case("admin"))
        {
            Role::Admin
        } else {
            Role::User
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    #[default]
    Active,
    Inactive,
    Invited,
    Suspended,
}

impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserStatus::Active => "active",
            UserStatus::Inactive => "inactive",
            UserStatus::Invited => "invited",
            UserStatus::Suspended => "suspended",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "active" => Some(Self::Active),
            "inactive" => Some(Self::Inactive),
            "invited" => Some(Self::Invited),
            "suspended" => Some(Self::Suspended),
            _ => None,
        }
    }
}

/// An account that can sign in to the admin UI and/or call the processing API.
#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub username: Option<String>,
    pub email: String,
    /// Argon2 PHC string. Never serialised.
    pub password_hash: String,
    pub role: Role,
    /// Role names assigned via the admin UI (descriptive; `role` is enforced).
    pub role_names: Vec<String>,
    pub status: UserStatus,
    /// Key presented as the `x-api-key` header for the processing endpoints.
    pub api_key: String,
    pub permissions: Vec<String>,
    /// Profile picture as a `data:` URL (in-memory; set via the profile page).
    pub avatar: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn roles(&self) -> Vec<String> {
        if self.role_names.is_empty() {
            vec![self.role.as_str().to_string()]
        } else {
            self.role_names.clone()
        }
    }

    pub fn effective_permissions(&self) -> Vec<String> {
        if self.role.is_full_access() {
            vec!["*".to_string()]
        } else {
            self.permissions.clone()
        }
    }

    /// Shape consumed by the frontend auth store (`AuthUser`).
    pub fn to_public_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "username": self.username.clone().unwrap_or_else(|| self.email.clone()),
            "email": self.email,
            "role": self.role.as_str(),
            "avatar": self.avatar,
            "permissions": self.effective_permissions(),
            "roles": self.roles(),
        })
    }

    /// Richer shape for the admin user-management screens (`GET /api/v1/users`).
    /// `roles` is expanded into `{id, name}` objects using the supplied lookup.
    pub fn to_admin_json(
        &self,
        role_id_by_name: &dyn Fn(&str) -> Option<u32>,
    ) -> serde_json::Value {
        let roles: Vec<serde_json::Value> = self
            .roles()
            .into_iter()
            .map(|name| serde_json::json!({ "id": role_id_by_name(&name), "name": name }))
            .collect();
        let created = self.created_at.to_rfc3339();
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "username": self.username,
            "email": self.email,
            "status": self.status.as_str(),
            "role": self.roles().first().cloned().unwrap_or_default(),
            "roles": roles,
            "avatar": self.avatar,
            "last_activity": null,
            "lastActivity": null,
            "created_at": created,
            "createdAt": created,
        })
    }
}

/// Opaque server-side session created on login, keyed by a random token.
#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

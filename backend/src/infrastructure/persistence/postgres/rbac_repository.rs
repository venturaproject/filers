//! Postgres RBAC catalogue (roles + permissions). Same trait contract as the
//! in-memory repos; seeded on first run by [`super::seed_rbac_if_missing`].

use async_trait::async_trait;
use sqlx::{PgPool, Row};

use crate::domain::rbac::{
    entities::{Permission, Role},
    repository::{
        NewPermission, NewRole, PermissionPatch, PermissionRepository, RolePatch, RoleRepository,
    },
};
use crate::errors::{AppError, AppResult};

fn map_err(e: sqlx::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!(e))
}

fn unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .and_then(|d| d.code())
        .is_some_and(|c| c == "23505")
}

// ── Permissions ─────────────────────────────────────────────────────────────

pub struct PgPermissionRepository {
    pool: PgPool,
}

impl PgPermissionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn row_to_permission(row: &sqlx::postgres::PgRow) -> Permission {
    Permission {
        id: row.get::<i32, _>("id") as u32,
        name: row.get("name"),
        guard_name: row.get("guard_name"),
        description: row.get("description"),
        created_at: row.get("created_at"),
    }
}

#[async_trait]
impl PermissionRepository for PgPermissionRepository {
    async fn list(&self) -> AppResult<Vec<Permission>> {
        let rows = sqlx::query(
            "SELECT id, name, guard_name, description, created_at FROM permissions ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.iter().map(row_to_permission).collect())
    }

    async fn find(&self, id: u32) -> AppResult<Option<Permission>> {
        let row = sqlx::query(
            "SELECT id, name, guard_name, description, created_at FROM permissions WHERE id = $1",
        )
        .bind(id as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.as_ref().map(row_to_permission))
    }

    async fn find_by_name(&self, name: &str) -> AppResult<Option<Permission>> {
        let row = sqlx::query(
            "SELECT id, name, guard_name, description, created_at FROM permissions WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.as_ref().map(row_to_permission))
    }

    async fn create(&self, input: NewPermission) -> AppResult<Permission> {
        let guard = if input.guard_name.is_empty() {
            "api".to_string()
        } else {
            input.guard_name
        };
        let row = sqlx::query(
            "INSERT INTO permissions (name, guard_name, description) VALUES ($1, $2, $3) \
             RETURNING id, name, guard_name, description, created_at",
        )
        .bind(&input.name)
        .bind(&guard)
        .bind(&input.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if unique_violation(&e) {
                AppError::Conflict(format!("permission '{}' already exists", input.name))
            } else {
                map_err(e)
            }
        })?;
        Ok(row_to_permission(&row))
    }

    async fn update(&self, id: u32, patch: PermissionPatch) -> AppResult<Permission> {
        let row = sqlx::query(
            "UPDATE permissions SET \
                name = COALESCE($2, name), \
                description = COALESCE($3, description) \
             WHERE id = $1 \
             RETURNING id, name, guard_name, description, created_at",
        )
        .bind(id as i32)
        .bind(&patch.name)
        .bind(&patch.description)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?
        .ok_or_else(|| AppError::NotFound(format!("Permission {id}")))?;
        Ok(row_to_permission(&row))
    }

    async fn delete(&self, id: u32) -> AppResult<()> {
        let res = sqlx::query("DELETE FROM permissions WHERE id = $1")
            .bind(id as i32)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Permission {id}")));
        }
        Ok(())
    }
}

// ── Roles ───────────────────────────────────────────────────────────────────

pub struct PgRoleRepository {
    pool: PgPool,
}

impl PgRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn row_to_role(row: &sqlx::postgres::PgRow) -> Role {
    Role {
        id: row.get::<i32, _>("id") as u32,
        name: row.get("name"),
        guard_name: row.get("guard_name"),
        description: row.get("description"),
        permission_ids: row
            .get::<Vec<i32>, _>("permission_ids")
            .into_iter()
            .map(|x| x as u32)
            .collect(),
        created_at: row.get("created_at"),
    }
}

#[async_trait]
impl RoleRepository for PgRoleRepository {
    async fn list(&self) -> AppResult<Vec<Role>> {
        let rows = sqlx::query(
            "SELECT id, name, guard_name, description, permission_ids, created_at \
             FROM roles ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows.iter().map(row_to_role).collect())
    }

    async fn find(&self, id: u32) -> AppResult<Option<Role>> {
        let row = sqlx::query(
            "SELECT id, name, guard_name, description, permission_ids, created_at \
             FROM roles WHERE id = $1",
        )
        .bind(id as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.as_ref().map(row_to_role))
    }

    async fn find_by_name(&self, name: &str) -> AppResult<Option<Role>> {
        let row = sqlx::query(
            "SELECT id, name, guard_name, description, permission_ids, created_at \
             FROM roles WHERE lower(name) = lower($1)",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.as_ref().map(row_to_role))
    }

    async fn create(&self, input: NewRole) -> AppResult<Role> {
        let guard = if input.guard_name.is_empty() {
            "api".to_string()
        } else {
            input.guard_name
        };
        let ids: Vec<i32> = input.permission_ids.iter().map(|&x| x as i32).collect();
        let row = sqlx::query(
            "INSERT INTO roles (name, guard_name, description, permission_ids) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, name, guard_name, description, permission_ids, created_at",
        )
        .bind(&input.name)
        .bind(&guard)
        .bind(&input.description)
        .bind(&ids)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if unique_violation(&e) {
                AppError::Conflict(format!("role '{}' already exists", input.name))
            } else {
                map_err(e)
            }
        })?;
        Ok(row_to_role(&row))
    }

    async fn update(&self, id: u32, patch: RolePatch) -> AppResult<Role> {
        let ids: Option<Vec<i32>> = patch
            .permission_ids
            .map(|v| v.into_iter().map(|x| x as i32).collect());
        let row = sqlx::query(
            "UPDATE roles SET \
                name = COALESCE($2, name), \
                description = COALESCE($3, description), \
                permission_ids = COALESCE($4, permission_ids) \
             WHERE id = $1 \
             RETURNING id, name, guard_name, description, permission_ids, created_at",
        )
        .bind(id as i32)
        .bind(&patch.name)
        .bind(&patch.description)
        .bind(&ids)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?
        .ok_or_else(|| AppError::NotFound(format!("Role {id}")))?;
        Ok(row_to_role(&row))
    }

    async fn delete(&self, id: u32) -> AppResult<()> {
        let name: Option<String> = sqlx::query("SELECT name FROM roles WHERE id = $1")
            .bind(id as i32)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?
            .map(|r| r.get("name"));

        match name {
            None => Err(AppError::NotFound(format!("Role {id}"))),
            Some(n) if n.eq_ignore_ascii_case("admin") => Err(AppError::Conflict(
                "the admin role cannot be deleted".into(),
            )),
            Some(_) => {
                sqlx::query("DELETE FROM roles WHERE id = $1")
                    .bind(id as i32)
                    .execute(&self.pool)
                    .await
                    .map_err(map_err)?;
                Ok(())
            }
        }
    }
}

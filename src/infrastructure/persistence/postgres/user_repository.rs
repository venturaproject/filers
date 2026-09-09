use async_trait::async_trait;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::auth::{
    entities::{Role, User, UserStatus},
    repository::UserRepository,
};
use crate::errors::{AppError, AppResult};

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(e: sqlx::Error) -> AppError {
    AppError::Internal(anyhow::anyhow!(e))
}

/// Translate a unique-violation into the same `Conflict` the memory repo raises.
fn map_write_err(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &e
        && db.is_unique_violation()
    {
        return AppError::Conflict("email or api key is already registered".into());
    }
    map_err(e)
}

fn row_to_user(row: PgRow) -> User {
    let role_names: Vec<String> = row.get("role_names");
    User {
        id: row.get("id"),
        name: row.get("name"),
        username: row.get("username"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        role: Role::from_role_names([row.get::<String, _>("role")]),
        role_names,
        status: UserStatus::parse(row.get::<String, _>("status").as_str())
            .unwrap_or(UserStatus::Active),
        api_key: row.get("api_key"),
        permissions: row.get("permissions"),
        avatar: row.get("avatar"),
        created_at: row.get("created_at"),
    }
}

const COLS: &str = "id, name, username, email, password_hash, role, role_names, status, \
                    api_key, permissions, avatar, created_at";

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn list(&self) -> AppResult<Vec<User>> {
        let rows = sqlx::query(&format!("SELECT {COLS} FROM users ORDER BY created_at"))
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows.into_iter().map(row_to_user).collect())
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        let row = sqlx::query(&format!("SELECT {COLS} FROM users WHERE id = $1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.map(row_to_user))
    }

    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(&format!("SELECT {COLS} FROM users WHERE email = $1"))
            .bind(email.trim().to_lowercase())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.map(row_to_user))
    }

    async fn find_by_api_key(&self, api_key: &str) -> AppResult<Option<User>> {
        let row = sqlx::query(&format!("SELECT {COLS} FROM users WHERE api_key = $1"))
            .bind(api_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.map(row_to_user))
    }

    async fn create(&self, user: User) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO users (id, name, username, email, password_hash, role, role_names, \
             status, api_key, permissions, avatar, created_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(user.id)
        .bind(&user.name)
        .bind(&user.username)
        .bind(user.email.to_lowercase())
        .bind(&user.password_hash)
        .bind(user.role.as_str())
        .bind(&user.role_names)
        .bind(user.status.as_str())
        .bind(&user.api_key)
        .bind(&user.permissions)
        .bind(&user.avatar)
        .bind(user.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn update(&self, user: User) -> AppResult<()> {
        let done = sqlx::query(
            "UPDATE users SET name=$2, username=$3, email=$4, password_hash=$5, role=$6, \
             role_names=$7, status=$8, api_key=$9, permissions=$10, avatar=$11 WHERE id=$1",
        )
        .bind(user.id)
        .bind(&user.name)
        .bind(&user.username)
        .bind(user.email.to_lowercase())
        .bind(&user.password_hash)
        .bind(user.role.as_str())
        .bind(&user.role_names)
        .bind(user.status.as_str())
        .bind(&user.api_key)
        .bind(&user.permissions)
        .bind(&user.avatar)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;

        if done.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User {}", user.id)));
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> AppResult<()> {
        let done = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        if done.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("User {id}")));
        }
        Ok(())
    }
}

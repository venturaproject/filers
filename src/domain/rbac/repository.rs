use async_trait::async_trait;

use super::entities::{Permission, Role};
use crate::errors::AppResult;

pub struct NewPermission {
    pub name: String,
    pub guard_name: String,
    pub description: Option<String>,
}

pub struct PermissionPatch {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn list(&self) -> AppResult<Vec<Permission>>;
    async fn find(&self, id: u32) -> AppResult<Option<Permission>>;
    async fn find_by_name(&self, name: &str) -> AppResult<Option<Permission>>;
    async fn create(&self, input: NewPermission) -> AppResult<Permission>;
    async fn update(&self, id: u32, patch: PermissionPatch) -> AppResult<Permission>;
    async fn delete(&self, id: u32) -> AppResult<()>;
}

pub struct NewRole {
    pub name: String,
    pub guard_name: String,
    pub description: Option<String>,
    pub permission_ids: Vec<u32>,
}

pub struct RolePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permission_ids: Option<Vec<u32>>,
}

#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn list(&self) -> AppResult<Vec<Role>>;
    async fn find(&self, id: u32) -> AppResult<Option<Role>>;
    async fn find_by_name(&self, name: &str) -> AppResult<Option<Role>>;
    async fn create(&self, input: NewRole) -> AppResult<Role>;
    async fn update(&self, id: u32, patch: RolePatch) -> AppResult<Role>;
    async fn delete(&self, id: u32) -> AppResult<()>;
}

use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;

use crate::domain::rbac::{
    entities::{Permission, Role},
    repository::{
        NewPermission, NewRole, PermissionPatch, PermissionRepository, RolePatch, RoleRepository,
    },
};
use crate::errors::{AppError, AppResult};

// ── Permissions ─────────────────────────────────────────────────────────────

pub struct MemoryPermissionRepository {
    store: DashMap<u32, Permission>,
    next_id: AtomicU32,
}

impl MemoryPermissionRepository {
    /// Seeds a `resource.action` catalogue and returns the repo plus a
    /// `name -> id` list so roles can be seeded against it.
    pub fn seeded() -> Self {
        let store = DashMap::new();
        let now = Utc::now();
        let mut id = 0u32;

        for name in crate::domain::rbac::catalogue_names() {
            id += 1;
            store.insert(
                id,
                Permission {
                    id,
                    name,
                    guard_name: "api".into(),
                    description: None,
                    created_at: now,
                },
            );
        }

        Self {
            store,
            next_id: AtomicU32::new(id + 1),
        }
    }

    pub fn ids_by_names(&self, names: &[&str]) -> Vec<u32> {
        self.store
            .iter()
            .filter(|p| names.contains(&p.name.as_str()))
            .map(|p| p.id)
            .collect()
    }

    pub fn all_ids(&self) -> Vec<u32> {
        self.store.iter().map(|p| p.id).collect()
    }
}

#[async_trait]
impl PermissionRepository for MemoryPermissionRepository {
    async fn list(&self) -> AppResult<Vec<Permission>> {
        let mut items: Vec<Permission> = self.store.iter().map(|p| p.clone()).collect();
        items.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(items)
    }

    async fn find(&self, id: u32) -> AppResult<Option<Permission>> {
        Ok(self.store.get(&id).map(|p| p.clone()))
    }

    async fn find_by_name(&self, name: &str) -> AppResult<Option<Permission>> {
        Ok(self
            .store
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.clone()))
    }

    async fn create(&self, input: NewPermission) -> AppResult<Permission> {
        if self.store.iter().any(|p| p.name == input.name) {
            return Err(AppError::Conflict(format!(
                "permission '{}' already exists",
                input.name
            )));
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let permission = Permission {
            id,
            name: input.name,
            guard_name: if input.guard_name.is_empty() {
                "api".into()
            } else {
                input.guard_name
            },
            description: input.description,
            created_at: Utc::now(),
        };
        self.store.insert(id, permission.clone());
        Ok(permission)
    }

    async fn update(&self, id: u32, patch: PermissionPatch) -> AppResult<Permission> {
        let mut entry = self
            .store
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("Permission {id}")))?;
        if let Some(name) = patch.name {
            entry.name = name;
        }
        if let Some(description) = patch.description {
            entry.description = Some(description);
        }
        Ok(entry.clone())
    }

    async fn delete(&self, id: u32) -> AppResult<()> {
        self.store
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| AppError::NotFound(format!("Permission {id}")))
    }
}

// ── Roles ───────────────────────────────────────────────────────────────────

pub struct MemoryRoleRepository {
    store: DashMap<u32, Role>,
    next_id: AtomicU32,
}

impl MemoryRoleRepository {
    pub fn seeded(admin_permission_ids: Vec<u32>, user_permission_ids: Vec<u32>) -> Self {
        let store = DashMap::new();
        let now = Utc::now();

        store.insert(
            1,
            Role {
                id: 1,
                name: "admin".into(),
                guard_name: "api".into(),
                description: Some("Full access to everything".into()),
                permission_ids: admin_permission_ids,
                created_at: now,
            },
        );
        store.insert(
            2,
            Role {
                id: 2,
                name: "user".into(),
                guard_name: "api".into(),
                description: Some("Can process files and view jobs".into()),
                permission_ids: user_permission_ids,
                created_at: now,
            },
        );

        Self {
            store,
            next_id: AtomicU32::new(3),
        }
    }
}

#[async_trait]
impl RoleRepository for MemoryRoleRepository {
    async fn list(&self) -> AppResult<Vec<Role>> {
        let mut items: Vec<Role> = self.store.iter().map(|r| r.clone()).collect();
        items.sort_by_key(|a| a.id);
        Ok(items)
    }

    async fn find(&self, id: u32) -> AppResult<Option<Role>> {
        Ok(self.store.get(&id).map(|r| r.clone()))
    }

    async fn find_by_name(&self, name: &str) -> AppResult<Option<Role>> {
        Ok(self
            .store
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
            .map(|r| r.clone()))
    }

    async fn create(&self, input: NewRole) -> AppResult<Role> {
        if self
            .store
            .iter()
            .any(|r| r.name.eq_ignore_ascii_case(&input.name))
        {
            return Err(AppError::Conflict(format!(
                "role '{}' already exists",
                input.name
            )));
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let role = Role {
            id,
            name: input.name,
            guard_name: if input.guard_name.is_empty() {
                "api".into()
            } else {
                input.guard_name
            },
            description: input.description,
            permission_ids: input.permission_ids,
            created_at: Utc::now(),
        };
        self.store.insert(id, role.clone());
        Ok(role)
    }

    async fn update(&self, id: u32, patch: RolePatch) -> AppResult<Role> {
        let mut entry = self
            .store
            .get_mut(&id)
            .ok_or_else(|| AppError::NotFound(format!("Role {id}")))?;
        if let Some(name) = patch.name {
            entry.name = name;
        }
        if let Some(description) = patch.description {
            entry.description = Some(description);
        }
        if let Some(ids) = patch.permission_ids {
            entry.permission_ids = ids;
        }
        Ok(entry.clone())
    }

    async fn delete(&self, id: u32) -> AppResult<()> {
        if let Some(role) = self.store.get(&id)
            && role.name.eq_ignore_ascii_case("admin")
        {
            return Err(AppError::Conflict(
                "the admin role cannot be deleted".into(),
            ));
        }
        self.store
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| AppError::NotFound(format!("Role {id}")))
    }
}

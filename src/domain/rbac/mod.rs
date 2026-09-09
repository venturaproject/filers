pub mod entities;
pub mod repository;

/// The seed `resource.action` permission catalogue. Inserted verbatim (in this
/// order) on first run by both the in-memory and the Postgres repositories, so
/// the ids line up across backends.
pub const CATALOGUE: &[(&str, &[&str])] = &[
    ("users", &["view", "create", "update", "delete"]),
    ("roles", &["view", "create", "update", "delete"]),
    ("permissions", &["view", "create", "update", "delete"]),
    ("api_clients", &["view", "create", "update", "delete"]),
    ("files", &["process", "batch"]),
    ("jobs", &["view"]),
    ("settings", &["view", "update"]),
];

/// Permissions granted to the seeded `user` role. The `admin` role gets all.
pub const USER_ROLE_PERMISSIONS: &[&str] = &["files.process", "files.batch", "jobs.view"];

/// Flattened `["users.view", "users.create", ...]` view of [`CATALOGUE`].
pub fn catalogue_names() -> impl Iterator<Item = String> {
    CATALOGUE
        .iter()
        .flat_map(|(resource, actions)| actions.iter().map(move |a| format!("{resource}.{a}")))
}

//! OpenAPI document for the public processing API.
//!
//! Only the endpoints an external integrator calls are described here — the
//! session-cookie admin routes under `/api/v1/*` are intentionally excluded.
//!
//! Served (spec + Scalar UI) only when [`crate::config::Config::enable_api_docs`]
//! is true; see [`super::router`].

use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::domain::processing::entities::{
    FileFormat, ParseError, ParseOptions, ParseStats, ParsedFile, Timings,
};

/// `multipart/form-data` body: a single spreadsheet upload.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct UploadForm {
    /// Spreadsheet to process — `.xlsx`, `.xls`, `.ods` or `.csv`.
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

/// `multipart/form-data` body: a JSON `schema` field plus the `file`.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct SchemaFileForm {
    /// Validation schema as a JSON string.
    pub schema: String,
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

/// `multipart/form-data` body: a JSON `spec` field plus the `file`.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct SpecFileForm {
    /// Transform spec as a JSON string.
    pub spec: String,
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

/// `multipart/form-data` body: a JSON `pipeline` field plus the `file`.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct PipelineFileForm {
    /// Pipeline definition as a JSON string (`{ "steps": [ ... ] }`).
    pub pipeline: String,
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

/// `multipart/form-data` body: two spreadsheets to compare.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct DiffForm {
    #[schema(value_type = String, format = Binary)]
    pub a: String,
    #[schema(value_type = String, format = Binary)]
    pub b: String,
}

/// JSON body for `POST /api/process/batch`.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct BatchBody {
    /// Sub-directory under `BATCH_BASE_DIR` to scan. `..` and absolute paths
    /// are rejected. Omit to scan the base directory itself.
    #[schema(example = "incoming/2026-02")]
    pub path: Option<String>,
    /// Parse options applied to every file in the batch.
    pub options: Option<ParseOptions>,
    /// URL to POST the job's final state to on completion (overrides
    /// `WEBHOOK_URL`).
    pub webhook_url: Option<String>,
    /// When set, each input is converted and written to a downloadable result
    /// file: `{ "to": "csv|json|ndjson|xlsx", "transform": { ... } }`.
    #[schema(value_type = Object)]
    pub output: Option<serde_json::Value>,
}

/// JSON body for the external OAuth2 token endpoint.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct TokenBody {
    pub client_id: String,
    pub client_secret: String,
}

/// JSON body for the refresh endpoint.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub struct RefreshBody {
    pub refresh_token: String,
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "x-api-key",
                "Static service API key.",
            ))),
        );
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some(
                        "OAuth2 client-credentials access token from `POST /api/ext/auth/token`.",
                    ))
                    .build(),
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Filers processing API",
        version = "0.1.0",
        description = "Fast spreadsheet parsing, validation, transformation and \
                       comparison. Authenticate with a static `x-api-key` header \
                       or an OAuth2 bearer token. `files:read` covers profile / \
                       validate / diff / job reads; `files:write` covers process \
                       / convert / transform / pipeline / batch.",
    ),
    servers((url = "/", description = "This host")),
    paths(
        crate::infrastructure::http::handlers::process::handle,
        crate::infrastructure::http::handlers::process_ops::profile,
        crate::infrastructure::http::handlers::process_ops::validate,
        crate::infrastructure::http::handlers::process_ops::convert,
        crate::infrastructure::http::handlers::process_ops::transform,
        crate::infrastructure::http::handlers::process_ops::diff,
        crate::infrastructure::http::handlers::process_ops::pipeline,
        crate::infrastructure::http::handlers::generate::xlsx,
        crate::infrastructure::http::handlers::batch::handle,
        crate::infrastructure::http::handlers::jobs::handle,
        crate::infrastructure::http::handlers::jobs::results,
        crate::infrastructure::http::handlers::jobs::result_file,
        crate::infrastructure::http::handlers::ext_auth::token,
        crate::infrastructure::http::handlers::ext_auth::refresh,
    ),
    components(schemas(
        ParsedFile, ParseOptions, ParseStats, ParseError, Timings, FileFormat,
        UploadForm, SchemaFileForm, SpecFileForm, PipelineFileForm, DiffForm,
        BatchBody, TokenBody, RefreshBody,
        crate::application::processing::operations::generate::XlsxRequest,
        crate::application::processing::operations::generate::XlsxOptions,
    )),
    tags(
        (name = "processing", description = "Synchronous operations on an uploaded file"),
        (name = "batch", description = "Asynchronous multi-file jobs"),
        (name = "jobs", description = "Job status and downloadable results"),
        (name = "auth", description = "OAuth2 client-credentials for external clients"),
    ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

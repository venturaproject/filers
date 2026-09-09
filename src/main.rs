use std::net::SocketAddr;

use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use rust_api::{bootstrap, config::Config};

#[derive(Parser)]
#[command(
    name = "server",
    version,
    about = "Filers processing API server + maintenance CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP server (this is the default with no subcommand).
    Serve,
    /// Print the effective configuration and run the startup checks.
    Check,
    /// Job-history maintenance.
    Jobs {
        #[command(subcommand)]
        action: JobsAction,
    },
}

#[derive(Subcommand)]
enum JobsAction {
    /// Delete completed / failed jobs older than N days (running jobs are kept).
    /// Schedule this from cron for durable retention.
    Prune {
        /// Age threshold in days.
        #[arg(long, default_value_t = 7)]
        days: i64,
        /// List what would be deleted without deleting anything.
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_api=info,tower_http=info".parse().expect("filter")),
        )
        .init();

    let cli = Cli::parse();
    let config = Config::from_env();

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(config).await,
        Command::Check => check(&config),
        Command::Jobs {
            action: JobsAction::Prune { days, dry_run },
        } => {
            if let Err(e) = prune_jobs(config, days, dry_run).await {
                eprintln!("prune failed: {e:#}");
                std::process::exit(1);
            }
        }
    }
}

async fn serve(config: Config) {
    let port = config.port;

    let (errors, warnings) = config.validate();
    for w in &warnings {
        tracing::warn!("config: {w}");
    }
    if !errors.is_empty() {
        for e in &errors {
            tracing::error!("config: {e}");
        }
        eprintln!(
            "\nRefusing to start in production with {} config error(s) above.\n\
             Fix them or unset APP_ENV=production to boot with warnings only.\n",
            errors.len()
        );
        std::process::exit(1);
    }

    tracing::info!(email = %config.seed_user.email, "seeded admin user");

    let app = bootstrap::build_app_async(config).await.expect("build app");

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("bind");

    tracing::info!("Listening on port {port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("serve");
}

fn check(config: &Config) {
    println!("port                 {}", config.port);
    println!(
        "app_env              {}",
        if config.production {
            "production"
        } else {
            "(dev)"
        }
    );
    println!(
        "database_url          {}",
        config
            .database_url
            .as_deref()
            .map(mask_url)
            .unwrap_or_else(|| "(unset — in-memory)".into())
    );
    println!("api_keys             {} configured", config.api_keys.len());
    println!("max_file_size_mb     {}", config.max_file_size_mb);
    println!("max_cells            {}", config.max_cells);
    println!("max_uncompressed_mb  {}", config.max_uncompressed_mb);
    println!("cors_origins         {}", config.cors_origins.join(", "));
    println!(
        "api_rate_limit       {}",
        config
            .api_rate_limit
            .map(|(n, w)| format!("{n}/{w}s"))
            .unwrap_or_else(|| "off".into())
    );
    println!("enable_api_docs      {}", config.enable_api_docs);
    println!("seed_demo_users      {}", config.seed_demo_users);
    println!(
        "webhook_url          {}",
        config.webhook_url.as_deref().unwrap_or("(unset)")
    );

    let (errors, warnings) = config.validate();
    for w in &warnings {
        println!("  warning: {w}");
    }
    for e in &errors {
        println!("  ERROR:   {e}");
    }
    if errors.is_empty() && warnings.is_empty() {
        println!("\nchecks passed.");
    } else if !errors.is_empty() {
        std::process::exit(1);
    }
}

/// Hide the password in a `postgres://user:pass@host/db` string.
fn mask_url(url: &str) -> String {
    match (url.find("://"), url.find('@')) {
        (Some(scheme), Some(at)) if at > scheme + 3 => {
            let creds = &url[scheme + 3..at];
            if let Some(colon) = creds.find(':') {
                format!(
                    "{}{}:***{}",
                    &url[..scheme + 3],
                    &creds[..colon],
                    &url[at..]
                )
            } else {
                url.to_string()
            }
        }
        _ => url.to_string(),
    }
}

async fn prune_jobs(config: Config, days: i64, dry_run: bool) -> anyhow::Result<()> {
    if config.database_url.is_none() {
        eprintln!(
            "DATABASE_URL is not set — job history is in-memory and nothing persists to prune."
        );
        return Ok(());
    }

    let cutoff = Utc::now() - Duration::days(days.max(0));
    let state = bootstrap::build_state_async(config).await?;
    let n = state
        .processing
        .jobs
        .prune_terminal(cutoff, dry_run)
        .await?;

    if dry_run {
        println!(
            "{n} completed/failed job(s) older than {days}d would be deleted (cutoff {cutoff})."
        );
    } else {
        println!("pruned {n} completed/failed job(s) older than {days}d (cutoff {cutoff}).");
    }
    Ok(())
}

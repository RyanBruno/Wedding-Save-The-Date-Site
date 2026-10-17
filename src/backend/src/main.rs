// `SsrMode::Static` renders the route through the fully-resolved `into_any()`
// path, whose type nesting is far deeper than the default limit allows for a
// page this long.
#![recursion_limit = "1024"]

mod admin;
mod api;
mod state;

use axum::Router;
use frontend::app::shell;
use leptos::config::get_configuration;
use leptos_axum::generate_route_list_with_ssg;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use admin::AdminAuth;
use state::AppState;

/// The site is statically generated, so this binary has two jobs:
///
/// 1. At start-up, prerender every route to flat HTML under `site_root`. That
///    directory plus `pkg/` and the images is the entire deployable site — it
///    is what the package installs for nginx to serve, and needs no server of
///    its own.
/// 2. Stay up to serve `/api`, the small dynamic surface a static host cannot
///    provide: RSVP replies, and the admin dashboard that reads them back.
///    Neither is part of the generated site — the dashboard in particular is
///    served from this binary and never written into `site_root`.
///
/// Pass `--generate-only` in a build pipeline to do (1) and exit.
#[tokio::main]
async fn main() {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ── Environment ───────────────────────────────────────────────────────────
    dotenvy::dotenv().ok();

    let generate_only = std::env::args().any(|arg| arg == "--generate-only");

    // ── Leptos config ─────────────────────────────────────────────────────────
    let conf = get_configuration(None).expect("Failed to read Leptos config");
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let site_root = leptos_options.site_root.to_string();

    // ── Static generation ─────────────────────────────────────────────────────
    // The generator renders whatever closure it is handed, so it has to be given
    // `shell` rather than `App`: handing it `App` alone produces a fragment with
    // no `<head>`, which leptos_meta then panics on.
    let (_routes, static_generator) = generate_route_list_with_ssg({
        let options = leptos_options.clone();
        move || shell(options.clone())
    });
    static_generator.generate(&leptos_options).await;
    tracing::info!("Prerendered site into {site_root}");

    if generate_only {
        return;
    }

    // ── Database ──────────────────────────────────────────────────────────────
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let connect_opts = database_url
        .parse::<SqliteConnectOptions>()
        .expect("Invalid DATABASE_URL")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_opts)
        .await
        .expect("Failed to connect to SQLite");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    // Read after `dotenvy` so `.env` can carry `ADMIN_PASSWORD`, and before the
    // listener binds so a missing password is logged at start-up rather than
    // discovered by someone staring at a 503.
    let state = AppState {
        db,
        admin: AdminAuth::from_env(),
    };

    // ── Axum router ───────────────────────────────────────────────────────────
    // `ServeDir` is here for local preview only. In production nginx serves the
    // generated directory out of the installed package and this process serves
    // `/api` alone, so nothing here decides what a cache may keep.
    let app = Router::new()
        .nest("/api", api::router())
        .fallback_service(ServeDir::new(&site_root).append_index_html_on_directories(true))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("API listening on http://{addr}/api");
    tracing::info!("Admin dashboard at http://{addr}/api/admin");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

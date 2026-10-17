use sqlx::SqlitePool;

use crate::admin::AdminAuth;

/// Shared application state threaded through the API handlers.
///
/// `LeptosOptions` used to live here for the SSR handler; with the site
/// statically generated at start-up, what is left is the database pool and the
/// admin dashboard's sign-in state.
///
/// `FromRef` is what lets the dashboard's session extractor reach `admin`
/// without every handler having to pass it along by hand.
#[derive(Clone, axum_macros::FromRef)]
pub struct AppState {
    pub db: SqlitePool,
    pub admin: AdminAuth,
}

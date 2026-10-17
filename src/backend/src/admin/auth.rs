//! Who is allowed into the dashboard, and how that is remembered.
//!
//! The guest-facing API deliberately has no accounts: it names people only when
//! handed a surname already on the list. The dashboard is the opposite — it
//! hands over the whole guest list — so it gets the one thing the rest of the
//! service does not have, a password.
//!
//! Deliberately not a user table. There is exactly one person who administers a
//! wedding guest list, the password lives in the environment beside
//! `DATABASE_URL`, and sessions live in memory: restarting the API signs the
//! browser out, which for a service that restarts on deploy is a feature.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::RngCore;
use subtle::ConstantTimeEq;

use super::error_body;

/// Cookie the browser carries back. Scoped to the dashboard's own prefix so it
/// is never sent with a guest's RSVP.
pub const COOKIE_NAME: &str = "wedding_admin";

/// Path the cookie is scoped to. Must match where the router is mounted.
pub const COOKIE_PATH: &str = "/api/admin";

/// How long a sign-in lasts. Long enough to work through a guest list in one
/// sitting, short enough that a forgotten tab does not stay open for a week.
const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Failed sign-ins tolerated before the lock-out below starts.
const FREE_ATTEMPTS: u32 = 5;

/// First lock-out, doubling with each further failure.
const LOCKOUT_BASE: Duration = Duration::from_secs(30);

/// Longest lock-out. A wrong password is usually a typo, so this stays short
/// enough to be survivable and long enough to make guessing pointless.
const LOCKOUT_MAX: Duration = Duration::from_secs(15 * 60);

/// Proof that the request carried a live session cookie.
///
/// An extractor rather than middleware: every protected handler simply names it
/// as an argument, so a new route cannot be added unguarded by forgetting to
/// wrap it in a layer.
pub struct AdminSession;

/// The sign-in state, shared by every handler through [`AppState`].
///
/// [`AppState`]: crate::state::AppState
#[derive(Clone)]
pub struct AdminAuth {
    /// `None` when `ADMIN_PASSWORD` is unset, which disables the dashboard
    /// outright. A blank password would otherwise publish the guest list to
    /// anyone who found the URL.
    password: Option<Arc<String>>,
    /// `Secure` on the session cookie. On by default; the only reason to turn
    /// it off is previewing over plain `http://` in a browser that refuses
    /// secure cookies on localhost.
    secure_cookie: bool,
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    /// Live tokens and when each expires.
    sessions: HashMap<String, Instant>,
    /// Consecutive failed sign-ins, reset by a successful one.
    failures: u32,
    locked_until: Option<Instant>,
}

/// Why a sign-in did not produce a session.
pub enum LoginRejected {
    /// `ADMIN_PASSWORD` is unset, so there is nothing to sign in to.
    Disabled,
    /// Wrong password.
    Wrong,
    /// Too many wrong passwords; how long is left.
    LockedOut(Duration),
}

impl AdminAuth {
    /// Reads the dashboard's configuration from the environment.
    ///
    /// Logs loudly when the dashboard is off, because "the page 503s" is
    /// otherwise a confusing way to discover a missing variable.
    pub fn from_env() -> Self {
        let password = std::env::var("ADMIN_PASSWORD")
            .ok()
            .map(|p| p.trim().to_owned())
            .filter(|p| !p.is_empty());

        match &password {
            Some(_) => tracing::info!("admin dashboard enabled at {COOKIE_PATH}"),
            None => tracing::warn!(
                "ADMIN_PASSWORD is unset — the admin dashboard is disabled and will answer 503."
            ),
        }

        // Anything other than an explicit "false" leaves the flag on, so a
        // typo in the variable name cannot quietly drop `Secure`.
        let secure_cookie = !matches!(
            std::env::var("ADMIN_COOKIE_INSECURE")
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes"
        );

        if !secure_cookie {
            tracing::warn!(
                "ADMIN_COOKIE_INSECURE is set — the admin session cookie will be sent over plain \
                 HTTP. Local previews only."
            );
        }

        Self {
            password: password.map(Arc::new),
            secure_cookie,
            inner: Arc::new(Mutex::new(Inner::default())),
        }
    }

    /// Whether a password was configured at all.
    pub fn enabled(&self) -> bool {
        self.password.is_some()
    }

    /// Checks a submitted password and, if it is right, mints a session.
    pub fn sign_in(&self, supplied: &str) -> Result<String, LoginRejected> {
        let Some(expected) = self.password.clone() else {
            return Err(LoginRejected::Disabled);
        };

        let now = Instant::now();
        let mut inner = self.lock();

        if let Some(until) = inner.locked_until {
            if until > now {
                return Err(LoginRejected::LockedOut(until - now));
            }
            inner.locked_until = None;
        }

        // Constant time: a byte-by-byte comparison that stops at the first
        // difference tells a caller how much of the password it has right.
        let correct: bool = supplied
            .as_bytes()
            .ct_eq(expected.as_bytes())
            .into();

        if !correct {
            inner.failures = inner.failures.saturating_add(1);
            if inner.failures > FREE_ATTEMPTS {
                let steps = inner.failures - FREE_ATTEMPTS - 1;
                let wait = LOCKOUT_BASE
                    .saturating_mul(2u32.saturating_pow(steps.min(10)))
                    .min(LOCKOUT_MAX);
                inner.locked_until = Some(now + wait);
                tracing::warn!(
                    "admin sign-in locked out for {}s after {} failures",
                    wait.as_secs(),
                    inner.failures
                );
                return Err(LoginRejected::LockedOut(wait));
            }
            tracing::warn!("admin sign-in failed ({} in a row)", inner.failures);
            return Err(LoginRejected::Wrong);
        }

        inner.failures = 0;
        inner.locked_until = None;

        let token = new_token();
        inner.sessions.retain(|_, expires| *expires > now);
        inner.sessions.insert(token.clone(), now + SESSION_TTL);

        tracing::info!("admin signed in ({} live session(s))", inner.sessions.len());
        Ok(token)
    }

    /// True when the token names a session that has not expired. Expired
    /// entries are dropped on the way past, which is the only sweep the store
    /// needs at this scale.
    fn is_live(&self, token: &str) -> bool {
        let now = Instant::now();
        let mut inner = self.lock();
        inner.sessions.retain(|_, expires| *expires > now);
        inner.sessions.contains_key(token)
    }

    /// Ends one session. Used by the sign-out button; other sessions, on other
    /// devices, are left alone.
    pub fn sign_out(&self, token: &str) {
        self.lock().sessions.remove(token);
    }

    /// `Set-Cookie` for a fresh session.
    pub fn cookie(&self, token: &str) -> String {
        let secure = if self.secure_cookie { "; Secure" } else { "" };
        format!(
            "{COOKIE_NAME}={token}; Path={COOKIE_PATH}; Max-Age={}; HttpOnly; SameSite=Strict{secure}",
            SESSION_TTL.as_secs()
        )
    }

    /// `Set-Cookie` that clears the session cookie.
    pub fn clearing_cookie(&self) -> String {
        let secure = if self.secure_cookie { "; Secure" } else { "" };
        format!("{COOKIE_NAME}=; Path={COOKIE_PATH}; Max-Age=0; HttpOnly; SameSite=Strict{secure}")
    }

    /// A poisoned mutex here means a previous handler panicked while holding
    /// it. Nothing in the guarded state can be left half-written — it is two
    /// counters and a map — so the lock is taken back rather than propagating
    /// the panic into every later request.
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Pulls the session token out of the request's `Cookie` header.
pub fn token_from(parts: &Parts) -> Option<String> {
    let header = parts.headers.get(header::COOKIE)?.to_str().ok()?;
    header
        .split(';')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, value)| (name.trim(), value.trim()))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value.to_owned())
}

/// Whether this request is already signed in — the question the HTML route
/// asks, since it answers with a login page rather than a 401.
pub fn signed_in(auth: &AdminAuth, parts: &Parts) -> bool {
    token_from(parts).is_some_and(|token| auth.is_live(&token))
}

impl<S> FromRequestParts<S> for AdminSession
where
    AdminAuth: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AdminAuth::from_ref(state);

        if !auth.enabled() {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(error_body("The admin dashboard is not configured.")),
            )
                .into_response());
        }

        if signed_in(&auth, parts) {
            return Ok(AdminSession);
        }

        // The dashboard's fetches treat this as "show the login screen again",
        // so it has to be distinguishable from an ordinary failure.
        Err((
            StatusCode::UNAUTHORIZED,
            Json(error_body("Your session has expired. Please sign in again.")),
        )
            .into_response())
    }
}

/// 32 random bytes, hex-encoded.
fn new_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        // Writing into a String cannot fail.
        let _ = write!(token, "{byte:02x}");
    }
    token
}

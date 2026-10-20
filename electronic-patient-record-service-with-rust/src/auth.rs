//! Secure-by-default bearer authentication.
//!
//! Login enforcement is ON unless a deployment says otherwise
//! (`settings.require_auth: false` in config, or `EPR_REQUIRE_AUTH=false`).
//! Tokens are PASETO v4.public, verified offline against the authentication
//! service's published Ed25519 keys. The service refuses to start when
//! enforcement is off in production, or on with no keys and no key URL.
//!
//! Not built yet: authorization (RBAC, ABAC, consent). A valid token is
//! currently enough. See tasks SE-1 to SE-4.

use authentication_verifier::Verifier;
use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use std::sync::OnceLock;

const DEFAULT_ISSUER: &str = "authentication-service";
const DEFAULT_AUDIENCE: &str = "electronic-patient-record";

/// Who is calling: the token's `sub`, or `anonymous` when login is off or the path is public.
/// The middleware puts one in every request's extensions; handlers use it for the audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent(pub String);

/// The agent recorded when no token was required.
pub const ANONYMOUS: &str = "anonymous";

static VERIFIER: OnceLock<Verifier> = OnceLock::new();
static REQUIRE_AUTH: OnceLock<bool> = OnceLock::new();

/// Paths that never need a token: health probes only.
#[must_use]
pub fn is_public_path(path: &str) -> bool {
    matches!(path, "/_health" | "/_ping" | "/_readiness")
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Resolve whether enforcement is on. An explicit `false` in the environment
/// wins, then the config value; anything else is on.
#[must_use]
pub fn resolve_require_auth(config: Option<bool>, env: Option<&str>) -> bool {
    match env.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
        Some("false" | "0" | "no" | "off") => false,
        Some("true" | "1" | "yes" | "on") => true,
        _ => config.unwrap_or(true),
    }
}

/// The startup gate, pure. `Err` means the service must refuse to start.
///
/// # Errors
///
/// Enforcement off in production; or on with no keys and no key URL.
pub fn check_startup(
    production: bool,
    require_auth: bool,
    keys: usize,
    keys_url_set: bool,
) -> Result<(), String> {
    if production && !require_auth {
        return Err("login enforcement is off in production; refusing to start".into());
    }
    if require_auth && keys == 0 && !keys_url_set {
        return Err("login enforcement is on but no token keys are configured \
             (set EPR_PASETO_KEYS or EPR_PASETO_KEYS_URL); refusing to start"
            .into());
    }
    Ok(())
}

/// Build a verifier from a key-set JSON value.
fn build(keys: &serde_json::Value) -> Result<Verifier, String> {
    let issuer = env_or("EPR_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("EPR_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    Verifier::from_paseto_keys_value(keys, &issuer, &audience).map_err(|e| e.to_string())
}

/// Initialise the process-wide guard from config and environment, fetching the
/// key set from `EPR_PASETO_KEYS_URL` when set. Call once before serving.
///
/// # Errors
///
/// When the startup gate refuses, or the key set cannot be built.
pub async fn init(production: bool, config_require_auth: Option<bool>) -> Result<(), String> {
    let require = resolve_require_auth(
        config_require_auth,
        std::env::var("EPR_REQUIRE_AUTH").ok().as_deref(),
    );
    let url = std::env::var("EPR_PASETO_KEYS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let inline = std::env::var("EPR_PASETO_KEYS")
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    let mut verifier = build(&inline)?;
    if let Some(url) = &url {
        let issuer = env_or("EPR_TOKEN_ISSUER", DEFAULT_ISSUER);
        let audience = env_or("EPR_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
        match Verifier::from_paseto_keys_url(url, &issuer, &audience).await {
            Ok(fetched) => verifier = fetched,
            Err(e) => {
                tracing::warn!(error = %e, "key set fetch failed; every token is rejected until keys load")
            }
        }
    }
    check_startup(production, require, verifier.key_count(), url.is_some())?;
    let _ = VERIFIER.set(verifier);
    let _ = REQUIRE_AUTH.set(require);
    Ok(())
}

/// The bearer token from the `Authorization` header.
fn bearer_token(headers: &HeaderMap) -> Result<&str, (StatusCode, String)> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing authorization header".to_string(),
        ))?;
    header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .map(str::trim)
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "expected bearer token".to_string(),
        ))
}

/// The decision, pure: `Ok(agent)` lets the request through; `Err((401, why))` rejects.
///
/// # Errors
///
/// `401` when enforcement is on, the path is not public, and the bearer token
/// is missing or fails verification.
pub fn enforce(
    require_auth: bool,
    path: &str,
    headers: &HeaderMap,
    verifier: &Verifier,
) -> Result<Agent, (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok(Agent(ANONYMOUS.to_string()));
    }
    verifier
        .verify(bearer_token(headers)?)
        .map(|claims| Agent(claims.sub))
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))
}

/// Axum middleware over the process-wide guard. Fails closed: before `init`
/// has run, every non-public request is rejected.
pub async fn require_auth_mw(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let decision = match (REQUIRE_AUTH.get(), VERIFIER.get()) {
        (Some(&require), Some(verifier)) => {
            enforce(require, req.uri().path(), req.headers(), verifier)
        }
        _ if is_public_path(req.uri().path()) => Ok(Agent(ANONYMOUS.to_string())),
        _ => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "authentication not initialised".to_string(),
        )),
    };
    match decision {
        Ok(agent) => {
            let mut req = req;
            req.extensions_mut().insert(agent);
            next.run(req).await
        }
        Err((status, msg)) => (status, msg).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn empty_verifier() -> Verifier {
        Verifier::from_paseto_keys_value(&serde_json::json!({ "keys": [] }), "i", "a").unwrap()
    }

    #[test]
    fn auth_is_on_by_default() {
        assert!(resolve_require_auth(None, None));
        assert!(!resolve_require_auth(Some(false), None));
        assert!(!resolve_require_auth(Some(true), Some("false")));
        assert!(resolve_require_auth(Some(false), Some("true")));
        assert!(resolve_require_auth(None, Some("garbage")));
    }

    #[test]
    fn startup_gate() {
        assert!(check_startup(true, false, 1, true).is_err());
        assert!(check_startup(false, true, 0, false).is_err());
        assert!(check_startup(false, true, 0, true).is_ok());
        assert!(check_startup(false, true, 1, false).is_ok());
        assert!(check_startup(false, false, 0, false).is_ok());
    }

    #[test]
    fn health_is_public_everything_else_needs_a_token() {
        let v = empty_verifier();
        let h = HeaderMap::new();
        assert_eq!(
            enforce(true, "/_health", &h, &v).unwrap(),
            Agent(ANONYMOUS.into())
        );
        assert_eq!(
            enforce(true, "/api/patients/compare", &h, &v)
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        assert!(enforce(false, "/api/patients/compare", &h, &v).is_ok());
    }

    #[test]
    fn bad_bearer_is_rejected() {
        let v = empty_verifier();
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert_eq!(
            enforce(true, "/api/x", &h, &v).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
        h.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer v4.public.not-a-token"),
        );
        assert_eq!(
            enforce(true, "/api/x", &h, &v).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }
}

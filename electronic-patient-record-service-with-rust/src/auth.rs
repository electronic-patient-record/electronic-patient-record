//! Secure-by-default bearer authentication.
//!
//! Login enforcement is ON unless a deployment says otherwise
//! (`settings.require_auth: false` in config, or `EPR_REQUIRE_AUTH=false`).
//! Two kinds of bearer token are accepted. A token starting `v4.` is a PASETO v4.public, verified
//! offline against the authentication service's published Ed25519 keys. Any other token is offered
//! to Keycloak, when `EPR_KEYCLOAK_URL` is set: its signature is checked against the realm's keys,
//! its issuer must be `{url}/realms/{realm}`, it must carry one of `EPR_KEYCLOAK_AUDIENCES`, and
//! an `email` claim must be verified. Both yield the same caller identity (`sub`).
//! The service refuses to start when enforcement is off in production, or on with no keys, no key
//! URL and no Keycloak.
//!
//! Authentication yields who the caller is (`Agent`) and what the token says about them
//! (`Subject`: the roles in its `role` attribute; Keycloak roles reach it through
//! `EPR_KEYCLOAK_ROLE_MAP_FILE`). What they may do is decided in [`crate::authz`] and
//! [`crate::policy`]. With login off there is no subject and no authorization either; that is
//! for development only and the service refuses to start that way in production.
//!
//! Not built yet: consent, break-the-glass grants, record-level decisions. See tasks SE-2 to SE-4.

use crate::policy::Subject;
use authentication_verifier::Verifier;
use authentication_verifier::keycloak::{KeycloakSettings, KeycloakVerifier};
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
static KEYCLOAK: OnceLock<KeycloakVerifier> = OnceLock::new();
static REQUIRE_AUTH: OnceLock<bool> = OnceLock::new();

/// Paths that never need a token: health probes, and `/metrics`, which answers 404 unless the
/// deployer turned it on (`EPR_METRICS`, see [`crate::metrics`]).
#[must_use]
pub fn is_public_path(path: &str) -> bool {
    matches!(path, "/_health" | "/_ping" | "/_readiness" | "/metrics")
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
/// Enforcement off in production; or on with no PASETO keys, no key URL and no Keycloak.
pub fn check_startup(
    production: bool,
    require_auth: bool,
    keys: usize,
    keys_url_set: bool,
    keycloak: bool,
) -> Result<(), String> {
    if production && !require_auth {
        return Err("login enforcement is off in production; refusing to start".into());
    }
    if require_auth && keys == 0 && !keys_url_set && !keycloak {
        return Err("login enforcement is on but no token keys are configured \
             (set EPR_PASETO_KEYS, EPR_PASETO_KEYS_URL or EPR_KEYCLOAK_URL); refusing to start"
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

/// The inline key set from `EPR_PASETO_KEYS`: none when unset or blank (compose passes `${VAR:-}` as an
/// empty string), and an error when set but not JSON, so a typo stops startup instead of leaving a
/// service that rejects every token.
fn inline_keys(raw: Option<&str>) -> Result<serde_json::Value, String> {
    match raw {
        None => Ok(serde_json::json!({ "keys": [] })),
        Some(raw) if raw.trim().is_empty() => Ok(serde_json::json!({ "keys": [] })),
        Some(raw) => {
            serde_json::from_str(raw).map_err(|e| format!("EPR_PASETO_KEYS is not valid JSON: {e}"))
        }
    }
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
    let inline = inline_keys(std::env::var("EPR_PASETO_KEYS").ok().as_deref())?;
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
    let keycloak = init_keycloak()?;
    check_startup(
        production,
        require,
        verifier.key_count(),
        url.is_some(),
        keycloak,
    )?;
    let _ = VERIFIER.set(verifier);
    let _ = REQUIRE_AUTH.set(require);
    Ok(())
}

/// Build the process-wide Keycloak verifier from `EPR_KEYCLOAK_*`, and say whether one exists.
///
/// Fails closed and loudly: `EPR_KEYCLOAK_URL` unset leaves Keycloak off; a half-configured or
/// invalid realm (no realm, no audiences, a non-HTTPS URL that is not loopback, a bad role map) is
/// an error the service refuses to start on, because the operator asked for Keycloak and would
/// otherwise get a service that quietly rejects everyone.
fn init_keycloak() -> Result<bool, String> {
    let Some(settings) = KeycloakSettings::from_env("EPR").map_err(|e| e.to_string())? else {
        return Ok(false);
    };
    let issuer = settings.expected_issuer();
    let verifier = KeycloakVerifier::new(settings).map_err(|e| e.to_string())?;
    tracing::info!(%issuer, "auth: Keycloak access tokens accepted");
    let _ = KEYCLOAK.set(verifier);
    Ok(true)
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

/// Whether login is enforced. Fails closed: before `init` has run, it is.
#[must_use]
pub fn authentication_enforced() -> bool {
    REQUIRE_AUTH.get().copied().unwrap_or(true)
}

/// The decision: `Ok(agent)` lets the request through; `Err((401, why))` rejects.
///
/// A token starting `v4.` is a PASETO and goes to `paseto`. Any other token goes to `keycloak` when
/// one is configured; when none is, it is refused. A PASETO-shaped token is never offered to
/// Keycloak and a Keycloak token is never offered to PASETO.
///
/// # Errors
///
/// `401` when enforcement is on, the path is not public, and the bearer token is missing, is of a
/// kind no configured verifier accepts, or fails verification.
pub async fn enforce(
    require_auth: bool,
    path: &str,
    headers: &HeaderMap,
    paseto: &Verifier,
    keycloak: Option<&KeycloakVerifier>,
) -> Result<Agent, (StatusCode, String)> {
    authenticate(require_auth, path, headers, paseto, keycloak)
        .await
        .map(|(agent, _)| agent)
}

/// [`enforce`], also returning the subject (roles) the token carries. Anonymous callers (login
/// off, or a public path) get an empty subject.
///
/// # Errors
///
/// As [`enforce`].
pub async fn authenticate(
    require_auth: bool,
    path: &str,
    headers: &HeaderMap,
    paseto: &Verifier,
    keycloak: Option<&KeycloakVerifier>,
) -> Result<(Agent, Subject), (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok((Agent(ANONYMOUS.to_string()), Subject::default()));
    }
    let token = bearer_token(headers)?;
    let unauthorized = |e: String| (StatusCode::UNAUTHORIZED, e);
    let claims = if token.starts_with("v4.") {
        paseto
            .verify(token)
            .map_err(|e| unauthorized(e.to_string()))?
    } else if let Some(keycloak) = keycloak {
        keycloak
            .verify(token)
            .await
            .map_err(|e| unauthorized(e.to_string()))?
    } else {
        return Err(unauthorized(
            "no verifier accepts this kind of token".into(),
        ));
    };
    let subject = Subject::from_attrs(&claims.attrs);
    Ok((Agent(claims.sub), subject))
}

/// Axum middleware over the process-wide guard. Fails closed: before `init`
/// has run, every non-public request is rejected.
pub async fn require_auth_mw(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    let decision = match (REQUIRE_AUTH.get(), VERIFIER.get()) {
        (Some(&require), Some(verifier)) => {
            authenticate(
                require,
                req.uri().path(),
                req.headers(),
                verifier,
                KEYCLOAK.get(),
            )
            .await
        }
        _ if is_public_path(req.uri().path()) => {
            Ok((Agent(ANONYMOUS.to_string()), Subject::default()))
        }
        _ => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "authentication not initialised".to_string(),
        )),
    };
    match decision {
        Ok((agent, subject)) => {
            let mut req = req;
            req.extensions_mut().insert(agent);
            req.extensions_mut().insert(subject);
            next.run(req).await
        }
        // JSON on /api, an OperationOutcome on /fhir (CQ-11).
        Err((status, msg)) => {
            // Nothing is audited before a caller is known: count it so it is not invisible.
            crate::metrics::UNAUDITED_REFUSALS.inc();
            crate::api_error::ApiError::new(status, msg).for_path(&path)
        }
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
    fn invalid_inline_keys_fail_startup() {
        assert_eq!(
            inline_keys(None).unwrap(),
            serde_json::json!({ "keys": [] })
        );
        assert!(inline_keys(Some(r#"{"keys":[]}"#)).is_ok());
        // A blank value is unset, so the startup gate gives its own message (CI: container job).
        assert_eq!(
            inline_keys(Some("")).unwrap(),
            serde_json::json!({ "keys": [] })
        );
        assert_eq!(
            inline_keys(Some("  ")).unwrap(),
            serde_json::json!({ "keys": [] })
        );
        let e = inline_keys(Some("{keys: oops")).unwrap_err();
        assert!(e.starts_with("EPR_PASETO_KEYS is not valid JSON"), "{e}");
    }

    #[test]
    fn startup_gate() {
        assert!(check_startup(true, false, 1, true, false).is_err());
        assert!(check_startup(false, true, 0, false, false).is_err());
        assert!(check_startup(false, true, 0, true, false).is_ok());
        assert!(check_startup(false, true, 1, false, false).is_ok());
        // Keycloak alone is enough to start with login on.
        assert!(check_startup(false, true, 0, false, true).is_ok());
        // ...but not in production with login off.
        assert!(check_startup(true, false, 0, false, true).is_err());
        assert!(check_startup(false, false, 0, false, false).is_ok());
    }

    #[tokio::test]
    async fn health_is_public_everything_else_needs_a_token() {
        let v = empty_verifier();
        let h = HeaderMap::new();
        assert_eq!(
            enforce(true, "/_health", &h, &v, None).await.unwrap(),
            Agent(ANONYMOUS.into())
        );
        assert_eq!(
            enforce(true, "/api/patients/compare", &h, &v, None)
                .await
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        assert!(
            enforce(false, "/api/patients/compare", &h, &v, None)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn bad_bearer_is_rejected() {
        let v = empty_verifier();
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert_eq!(
            enforce(true, "/api/x", &h, &v, None).await.unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
        h.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer v4.public.not-a-token"),
        );
        assert_eq!(
            enforce(true, "/api/x", &h, &v, None).await.unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }
}

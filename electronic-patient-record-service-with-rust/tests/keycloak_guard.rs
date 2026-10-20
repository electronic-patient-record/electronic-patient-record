//! A Keycloak access token through the real guard (`auth::enforce`): signature, issuer, audience,
//! expiry, verified email, and the routing rule between PASETO and Keycloak tokens.
//!
//! DB-free: a local OIDC provider (`authentication_verifier::test_idp`) stands in for Keycloak. It
//! proves the guard against a Keycloak-*shaped* realm, not against a running Keycloak; the
//! `keycloak` compose profile is how to try the real thing by hand.

use authentication_verifier::Verifier;
use authentication_verifier::keycloak::{KeycloakSettings, KeycloakVerifier};
use authentication_verifier::test_idp::{AUDIENCE, REALM, TestIdp};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use electronic_patient_record::auth::{self, Agent};
use serde_json::json;

const SUB: &str = "0c4f1e2a-0000-4000-8000-000000000001";

fn bearer(token: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    h
}

fn no_paseto_keys() -> Verifier {
    Verifier::from_paseto_keys_value(&json!({ "keys": [] }), "i", "a").unwrap()
}

async fn keycloak(idp: &TestIdp) -> KeycloakVerifier {
    KeycloakVerifier::new(KeycloakSettings {
        url: idp.base_url.clone(),
        realm: REALM.to_string(),
        audiences: vec![AUDIENCE.to_string()],
        role_map: Default::default(),
        require_verified_email: true,
    })
    .expect("settings are valid")
}

async fn decide(
    keycloak: Option<&KeycloakVerifier>,
    headers: &HeaderMap,
) -> Result<Agent, StatusCode> {
    auth::enforce(
        true,
        "/api/patients/compare",
        headers,
        &no_paseto_keys(),
        keycloak,
    )
    .await
    .map_err(|(code, _)| code)
}

#[tokio::test]
async fn a_valid_keycloak_token_is_accepted_and_identifies_the_caller() {
    let idp = TestIdp::start().await;
    let kc = keycloak(&idp).await;
    let agent = decide(Some(&kc), &bearer(&idp.token(|_| {})))
        .await
        .unwrap();
    assert_eq!(
        agent,
        Agent(SUB.to_string()),
        "the audit agent is the token's sub"
    );
}

#[tokio::test]
async fn a_token_with_no_roles_still_authenticates_because_roles_are_not_consulted_yet() {
    let idp = TestIdp::start().await;
    let kc = keycloak(&idp).await;
    let token = idp.token(|c| c["realm_access"] = json!({ "roles": [] }));
    assert!(decide(Some(&kc), &bearer(&token)).await.is_ok());
}

#[tokio::test]
async fn bad_keycloak_credentials_are_401_never_accepted() {
    let idp = TestIdp::start().await;
    let kc = keycloak(&idp).await;
    let cases = [
        ("no credential", HeaderMap::new()),
        (
            "expired",
            bearer(&idp.token(|c| c["exp"] = json!(1_000_000_000))),
        ),
        (
            "wrong audience",
            bearer(&idp.token(|c| c["aud"] = json!(["other-client"]))),
        ),
        (
            "wrong issuer",
            bearer(&idp.token(|c| c["iss"] = json!("https://evil.example/realms/mxi"))),
        ),
        (
            "unverified email",
            bearer(&idp.token(|c| c["email_verified"] = json!(false))),
        ),
        ("garbage", bearer("garbage")),
        ("not a bearer", {
            let mut h = HeaderMap::new();
            h.insert(AUTHORIZATION, "Basic abc".parse().unwrap());
            h
        }),
    ];
    for (name, headers) in cases {
        assert_eq!(
            decide(Some(&kc), &headers).await,
            Err(StatusCode::UNAUTHORIZED),
            "{name}"
        );
    }
}

#[tokio::test]
async fn a_token_signed_by_another_realm_is_refused() {
    // Two providers: the guard trusts only the first one's keys.
    let trusted = TestIdp::start().await;
    let other = TestIdp::start().await;
    let kc = keycloak(&trusted).await;
    // `other` signs with the same published test key but a different issuer URL, so the issuer check refuses it.
    assert_eq!(
        decide(Some(&kc), &bearer(&other.token(|_| {}))).await,
        Err(StatusCode::UNAUTHORIZED)
    );
}

#[tokio::test]
async fn without_keycloak_configured_a_non_paseto_token_is_refused() {
    let idp = TestIdp::start().await;
    assert_eq!(
        decide(None, &bearer(&idp.token(|_| {}))).await,
        Err(StatusCode::UNAUTHORIZED)
    );
}

#[tokio::test]
async fn a_paseto_shaped_token_is_never_offered_to_keycloak() {
    let idp = TestIdp::start().await;
    let kc = keycloak(&idp).await;
    // It goes to the PASETO verifier, which has no keys and refuses it. Even if it were a valid
    // JWT with a `v4.` prefix bolted on, Keycloak would not see it.
    let jwt_with_prefix = format!("v4.{}", idp.token(|_| {}));
    assert_eq!(
        decide(Some(&kc), &bearer("v4.public.AAAA")).await,
        Err(StatusCode::UNAUTHORIZED)
    );
    assert_eq!(
        decide(Some(&kc), &bearer(&jwt_with_prefix)).await,
        Err(StatusCode::UNAUTHORIZED)
    );
}

#[tokio::test]
async fn health_stays_public_and_login_off_is_anonymous() {
    let idp = TestIdp::start().await;
    let kc = keycloak(&idp).await;
    let none = HeaderMap::new();
    let h = auth::enforce(true, "/_health", &none, &no_paseto_keys(), Some(&kc))
        .await
        .unwrap();
    assert_eq!(h, Agent(auth::ANONYMOUS.to_string()));
    let off = auth::enforce(false, "/api/x", &none, &no_paseto_keys(), Some(&kc))
        .await
        .unwrap();
    assert_eq!(off, Agent(auth::ANONYMOUS.to_string()));
}

#[tokio::test]
async fn keycloak_must_be_https_unless_loopback_and_must_name_an_audience() {
    let bad = |url: &str, audiences: Vec<String>| {
        KeycloakVerifier::new(KeycloakSettings {
            url: url.to_string(),
            realm: "mxi".into(),
            audiences,
            role_map: Default::default(),
            require_verified_email: true,
        })
        .is_err()
    };
    assert!(
        bad("http://keycloak.example.com", vec!["a".into()]),
        "plain http to a remote host"
    );
    assert!(
        bad("https://keycloak.example.com", vec![]),
        "no audience would accept any client's token"
    );
}

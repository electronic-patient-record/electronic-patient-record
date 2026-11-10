//! Every error body the service sends, defined once (CQ-11). The REST API answers an RFC 9457
//! `application/problem+json` body: `detail` holds the message, and a field a client acts on sits beside
//! it as an extension, as `errors` does on a refused record (422). The FHIR facade answers an
//! `OperationOutcome`. Handlers and middleware build an [`ApiError`] and never a body by hand
//! (`tests/error_census.rs`); [`ApiError::for_path`] picks the shape for the route.

use crate::patients_store::StoreError;
use axa::problem::Problem;
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Map, Value};

/// The `detail` of a refused record: the reasons themselves are in `errors`.
const REFUSED: &str = "the record was refused";

/// A refused or failed request.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// A problem with `status`, `message` as its `detail`, and any `extra` fields.
    Error {
        /// The HTTP status.
        status: StatusCode,
        /// The `detail` text: fixed wording, never a database message or patient data.
        message: String,
        /// Fields beside `detail`, such as `current_version` or `break_glass_offer`.
        extra: Map<String, Value>,
    },
    /// 422 with an `errors` list: the record was refused, for each of these reasons.
    Invalid {
        /// The problems, one line each.
        problems: Vec<String>,
        /// Further keys in the body (`conflict_codes` on a refused merge).
        extra: Map<String, Value>,
    },
}

impl ApiError {
    /// A problem with `status` and `message` as its `detail`.
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self::Error {
            status,
            message: message.into(),
            extra: Map::new(),
        }
    }

    /// Add a field beside `detail`. A name RFC 9457 reserves (`type`, `title`, `status`, `detail`,
    /// `instance`) is dropped by [`Problem::with_extension`], so never use one.
    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        let (Self::Error { extra, .. } | Self::Invalid { extra, .. }) = &mut self;
        extra.insert(key.to_string(), value.into());
        self
    }

    /// 400 with `message`.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    /// 401: no usable credentials.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }

    /// 403 "forbidden".
    #[must_use]
    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden")
    }

    /// 404 "not found".
    #[must_use]
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not found")
    }

    /// 409 with `message`.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    /// 500 "internal error". The cause is logged by the caller, never sent.
    #[must_use]
    pub fn internal() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }

    /// 422 with each reason the record was refused.
    #[must_use]
    pub fn invalid(problems: Vec<String>) -> Self {
        Self::Invalid {
            problems,
            extra: Map::new(),
        }
    }

    /// The HTTP status.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Error { status, .. } => *status,
            Self::Invalid { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    /// The REST answer. `type` stays `about:blank`, so `title` is the status's standard reason, and
    /// the service's own wording is in `detail`.
    #[must_use]
    pub fn problem(&self) -> Problem {
        let (detail, extra) = match self {
            Self::Error { message, extra, .. } => (message.as_str(), extra),
            Self::Invalid { extra, .. } => (REFUSED, extra),
        };
        let mut problem = Problem::new(self.status()).with_detail(detail);
        if let Self::Invalid { problems, .. } = self {
            problem = problem.with_extension("errors", serde_json::json!(problems));
        }
        extra.iter().fold(problem, |problem, (key, value)| {
            problem.with_extension(key, value.clone())
        })
    }

    /// The same refusal as a FHIR `OperationOutcome` with one issue.
    #[must_use]
    pub fn into_fhir_response(self) -> Response {
        let status = self.status();
        let code = match status.as_u16() {
            401 => "login",
            403 => "forbidden",
            404 => "not-found",
            409 => "conflict",
            400 | 422 => "invalid",
            _ => "exception",
        };
        let diagnostics = match &self {
            Self::Error { message, .. } => message.clone(),
            Self::Invalid { problems, .. } => problems.join("; "),
        };
        let severity = if status.is_server_error() {
            "fatal"
        } else {
            "error"
        };
        let body = serde_json::json!({
            "resourceType": "OperationOutcome",
            "issue": [{ "severity": severity, "code": code, "diagnostics": diagnostics }],
        });
        let mut response = (
            status,
            [(header::CONTENT_TYPE, "application/fhir+json")],
            body.to_string(),
        )
            .into_response();
        retry_later(&mut response);
        response
    }

    /// The shape the route expects: an `OperationOutcome` under `/fhir/`, JSON everywhere else.
    #[must_use]
    pub fn for_path(self, path: &str) -> Response {
        if path.starts_with("/fhir/") {
            self.into_fhir_response()
        } else {
            self.into_response()
        }
    }
}

/// How long a client is told to wait after a 503, as `axa` 0.11 tells it for its own.
pub const RETRY_AFTER_SECONDS: u64 = 5;

/// A 503 says when to try again; nothing else does.
fn retry_later(response: &mut Response) {
    if response.status() == StatusCode::SERVICE_UNAVAILABLE {
        response.headers_mut().insert(
            header::RETRY_AFTER,
            header::HeaderValue::from(RETRY_AFTER_SECONDS),
        );
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = self.problem().into_response();
        retry_later(&mut response);
        response
    }
}

impl From<ApiError> for Box<Response> {
    fn from(e: ApiError) -> Self {
        Box::new(e.into_response())
    }
}

impl From<&StoreError> for ApiError {
    /// A miss is a 404, a stale version a 409 carrying `current_version`, a refused query a 400, a merge that is not
    /// the same person a 422 with `conflict_codes`, and a temporary identity dropped with no reason a 422 with `error_codes`. A database failure is logged and answered as a plain 500: its text
    /// is never sent.
    fn from(e: &StoreError) -> Self {
        match e {
            StoreError::NotFound => Self::not_found(),
            StoreError::Conflict { current } => {
                Self::conflict("version conflict").with("current_version", *current)
            }
            StoreError::BadQuery(m) => Self::bad_request(m.clone()),
            StoreError::NotSamePerson {
                message,
                conflict_codes,
            } => {
                Self::invalid(vec![message.clone()]).with("conflict_codes", conflict_codes.clone())
            }
            StoreError::TemporaryIdentityCleared => Self::invalid(vec![e.to_string()])
                .with("error_codes", ["temporary_identity_reason_required"]),
            StoreError::Db(m) => {
                tracing::error!(error = %m, "store failed");
                Self::internal()
            }
            StoreError::Unavailable(m) => {
                tracing::warn!(error = %m, "store unavailable");
                Self::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "the record cannot be reached just now; try again shortly",
                )
                .with("code", "unavailable")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(e: &ApiError) -> Value {
        serde_json::to_value(e.problem()).unwrap()
    }

    #[test]
    fn a_refused_merge_carries_its_reasons_as_codes_beside_the_errors() {
        let e = ApiError::from(&StoreError::NotSamePerson {
            message: "these do not look like the same person".into(),
            conflict_codes: serde_json::json!([{ "code": "different_nhs_numbers" }]),
        });
        assert_eq!(e.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body(&e),
            serde_json::json!({
                "type": "about:blank",
                "title": "Unprocessable Entity",
                "status": 422,
                "detail": "the record was refused",
                "errors": ["these do not look like the same person"],
                "conflict_codes": [{ "code": "different_nhs_numbers" }],
            })
        );
    }

    #[test]
    fn rest_bodies_have_the_shapes_the_ui_reads() {
        let forbidden = ApiError::forbidden().with("break_glass_offer", true);
        assert_eq!(
            body(&forbidden),
            serde_json::json!({
                "type": "about:blank",
                "title": "Forbidden",
                "status": 403,
                "detail": "forbidden",
                "break_glass_offer": true,
            })
        );
        assert_eq!(
            body(&ApiError::invalid(vec!["a".into()]))["errors"],
            serde_json::json!(["a"])
        );
        assert_eq!(
            ApiError::invalid(vec![]).status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        let stale = body(&ApiError::from(&StoreError::Conflict { current: 3 }));
        assert_eq!(stale["detail"], "version conflict");
        assert_eq!(stale["current_version"], 3);
    }

    #[test]
    fn a_database_failure_says_nothing_of_its_cause() {
        let failed = body(&ApiError::from(&StoreError::Db("secret detail".into())));
        assert_eq!(failed["detail"], "internal error");
        assert!(!failed.to_string().contains("secret"));
    }

    #[tokio::test]
    async fn a_database_that_cannot_serve_is_a_503_that_says_when_to_try_again() {
        let e = ApiError::from(&StoreError::Unavailable(
            "pool timed out at 10.0.0.5".into(),
        ));
        assert_eq!(e.status(), StatusCode::SERVICE_UNAVAILABLE);
        let b = body(&e);
        assert_eq!(b["code"], "unavailable");
        assert!(!b.to_string().contains("10.0.0.5"), "no cause: {b}");
        for path in ["/api/patients", "/fhir/R4/Patient"] {
            let r = ApiError::from(&StoreError::Unavailable("x".into())).for_path(path);
            assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(r.headers()[header::RETRY_AFTER], "5", "{path}");
        }
        let r = ApiError::from(&StoreError::Db("x".into())).for_path("/api/x");
        assert!(
            r.headers().get(header::RETRY_AFTER).is_none(),
            "a 500 says no such thing"
        );
    }

    #[tokio::test]
    async fn a_fhir_path_gets_an_operation_outcome() {
        let r = ApiError::forbidden().for_path("/fhir/R4/Patient/x");
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_eq!(r.headers()[header::CONTENT_TYPE], "application/fhir+json");
        let bytes = axum::body::to_bytes(r.into_body(), 1 << 16).await.unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["resourceType"], "OperationOutcome");
        assert_eq!(v["issue"][0]["code"], "forbidden");

        let r = ApiError::forbidden().for_path("/api/patients");
        assert_eq!(
            r.headers()[header::CONTENT_TYPE],
            "application/problem+json"
        );
    }
}

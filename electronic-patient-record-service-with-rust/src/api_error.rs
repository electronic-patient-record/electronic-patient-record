//! Every error body the service sends, defined once (CQ-11). The REST API answers
//! `{"error": message, …}` or, for a refused record, `{"errors": [problem, …]}` with 422; the FHIR
//! facade answers an `OperationOutcome`. Handlers and middleware build an [`ApiError`] and never a
//! body by hand (`tests/error_census.rs`); [`ApiError::for_path`] picks the shape for the route.

use crate::patients_store::StoreError;
use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde_json::{Map, Value};

/// A refused or failed request.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// `{"error": message}` plus any `extra` fields, with `status`.
    Error {
        /// The HTTP status.
        status: StatusCode,
        /// The `error` text: fixed wording, never a database message or patient data.
        message: String,
        /// Fields beside `error`, such as `current_version` or `break_glass_offer`.
        extra: Map<String, Value>,
    },
    /// 422 `{"errors": [...]}`: the record was refused, for each of these reasons.
    Invalid(Vec<String>),
}

impl ApiError {
    /// `{"error": message}` with `status`.
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self::Error {
            status,
            message: message.into(),
            extra: Map::new(),
        }
    }

    /// Add a field beside `error` (ignored for [`ApiError::Invalid`]).
    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        if let Self::Error { extra, .. } = &mut self {
            extra.insert(key.to_string(), value.into());
        }
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
        Self::Invalid(problems)
    }

    /// The HTTP status.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Error { status, .. } => *status,
            Self::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    /// The REST body.
    #[must_use]
    pub fn body(&self) -> Value {
        match self {
            Self::Error { message, extra, .. } => {
                let mut body = Map::new();
                body.insert("error".into(), Value::String(message.clone()));
                body.extend(extra.clone());
                Value::Object(body)
            }
            Self::Invalid(problems) => serde_json::json!({ "errors": problems }),
        }
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
            Self::Invalid(problems) => problems.join("; "),
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
        (
            status,
            [(header::CONTENT_TYPE, "application/fhir+json")],
            body.to_string(),
        )
            .into_response()
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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status(), Json(self.body())).into_response()
    }
}

impl From<ApiError> for Box<Response> {
    fn from(e: ApiError) -> Self {
        Box::new(e.into_response())
    }
}

impl From<&StoreError> for ApiError {
    fn from(e: &StoreError) -> Self {
        match e {
            StoreError::NotFound => Self::not_found(),
            StoreError::Conflict { current } => {
                Self::conflict("version conflict").with("current_version", *current)
            }
            StoreError::BadQuery(m) => Self::bad_request(m.clone()),
            StoreError::Db(m) => {
                tracing::error!(error = %m, "store failed");
                Self::internal()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_bodies_have_the_shapes_the_ui_reads() {
        assert_eq!(
            ApiError::forbidden().with("break_glass_offer", true).body(),
            serde_json::json!({"error": "forbidden", "break_glass_offer": true})
        );
        assert_eq!(
            ApiError::invalid(vec!["a".into()]).body(),
            serde_json::json!({"errors": ["a"]})
        );
        assert_eq!(
            ApiError::invalid(vec![]).status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ApiError::from(&StoreError::Conflict { current: 3 }).body(),
            serde_json::json!({"error": "version conflict", "current_version": 3})
        );
        assert_eq!(
            ApiError::from(&StoreError::Db("secret detail".into())).body(),
            serde_json::json!({"error": "internal error"})
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
        assert_eq!(r.headers()[header::CONTENT_TYPE], "application/json");
    }
}

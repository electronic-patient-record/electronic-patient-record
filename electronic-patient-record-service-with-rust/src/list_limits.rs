//! One explicit upper bound for JSON arrays that do not have a smaller domain bound (TL-13).
//!
//! The OpenAPI builder puts this value on every otherwise-unbounded array schema. This middleware
//! applies the same bound recursively to JSON request bodies before a handler can write anything.

use crate::api_error::ApiError;
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use serde_json::Value;

/// The maximum number of members in an otherwise-unbounded JSON array.
pub const MAX_ITEMS: usize = 1_000;

fn is_json(request: &Request) -> bool {
    request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.trim().ends_with("json"))
}

/// The first array over [`MAX_ITEMS`], as a dotted field path.
fn first_too_long(value: &Value, path: &str) -> Option<String> {
    match value {
        Value::Array(items) => {
            if items.len() > MAX_ITEMS {
                return Some(path.to_string());
            }
            items.iter().enumerate().find_map(|(index, item)| {
                let child = format!("{path}[{index}]");
                first_too_long(item, &child)
            })
        }
        Value::Object(fields) => fields.iter().find_map(|(field, value)| {
            let child = if path.is_empty() {
                field.clone()
            } else {
                format!("{path}.{field}")
            };
            first_too_long(value, &child)
        }),
        _ => None,
    }
}

/// Refuse an over-limit JSON array before a handler sees the request.
///
/// Malformed JSON is put back unchanged for the standard extractor to describe. The service's
/// outer body-size layer has already bounded how many bytes this middleware can read.
pub async fn enforce_mw(request: Request, next: Next) -> Response {
    if !is_json(&request) {
        return next.run(request).await;
    }

    let (parts, body) = request.into_parts();
    let path = parts.uri.path().to_string();
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "request body is too large")
                .for_path(&path);
        }
    };
    if let Ok(value) = serde_json::from_slice::<Value>(&bytes)
        && let Some(field) = first_too_long(&value, "")
    {
        return ApiError::invalid(vec![format!(
            "{field} may contain at most {MAX_ITEMS} items"
        )])
        .with("field", field)
        .with("max_items", MAX_ITEMS)
        .for_path(&path);
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Json, Router, middleware::from_fn, routing::post};
    use tower::ServiceExt;

    async fn answer(Json(body): Json<Value>) -> Json<Value> {
        Json(body)
    }

    fn router() -> Router {
        Router::new()
            .route("/api/test", post(answer))
            .layer(from_fn(enforce_mw))
    }

    async fn send(body: Value) -> Response {
        router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/test")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn an_array_at_the_limit_reaches_the_handler_unchanged() {
        let value = serde_json::json!({"patient": {"names": vec!["x"; MAX_ITEMS]}});
        let response = send(value.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), value);
    }

    #[tokio::test]
    async fn an_array_over_the_limit_is_422_and_names_the_field() {
        let response = send(serde_json::json!({
            "patient": {"names": [{"given": vec!["x"; MAX_ITEMS + 1]}]}
        }))
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["field"], "patient.names[0].given");
        assert_eq!(body["max_items"], MAX_ITEMS);
        assert_eq!(
            body["errors"][0],
            format!("patient.names[0].given may contain at most {MAX_ITEMS} items")
        );
    }
}

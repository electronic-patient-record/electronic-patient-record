//! Patient photo routes (the banner photo, task ID-7): `GET`/`PUT /api/patients/{id}/photo` and
//! `POST /api/patients/{id}/photo/withdraw`.
//!
//! A photo is identity data: every route goes through the same `authorize` as the identity record
//! (role, relationship, purpose of use, consent) and writes an audit row. A write and its audit
//! row are one transaction. The audit row never holds the image, only its size and type.

use super::common::{Access, audit_resource, authorize, begin, commit, purpose_of, store_failure};
use crate::api_error::ApiError;
use crate::audit::Action;
use crate::auth::Agent;
use crate::patients_store::StoreError;
use crate::photos_store::{self as store, StoredPhoto};
use crate::policy::{self, Resource, Subject};
use axum::{
    Extension,
    body::Bytes,
    extract::{Path, Query},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;

/// Which photo version the writer last saw (0 for none).
#[derive(Debug, serde::Deserialize)]
pub struct VersionQuery {
    /// Required: a write without it is a 400.
    pub expected_version: Option<i32>,
    /// Why, for the version history.
    pub reason: Option<String>,
}

fn image_response(photo: &StoredPhoto) -> Option<Response> {
    let (content_type, bytes) = (photo.content_type.as_deref()?, photo.bytes.as_ref()?);
    let mut r = (StatusCode::OK, Bytes::from(bytes.clone())).into_response();
    let h = r.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type).ok()?,
    );
    // The bytes are checked on the way in, but a browser must still never treat them as anything
    // but an image, and must not keep a patient's face in a shared cache.
    h.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; sandbox"),
    );
    h.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline; filename=\"photo\""),
    );
    h.insert(
        header::ETAG,
        HeaderValue::from_str(&format!("W/\"{}\"", photo.version)).ok()?,
    );
    Some(r)
}

pub(super) async fn read(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let purpose = match purpose_of(&headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::PatientIdentity,
        policy::Action::Read,
        purpose,
        Some(&id),
    )
    .await
    {
        return r.into_response();
    }
    let found = match store::current(&ctx.db, &id).await {
        Ok(p) => p,
        Err(e) => return store_failure(&e),
    };
    let patient_id = found.as_ref().map(|p| p.patient_id.clone());
    let photo = found.filter(|p| p.bytes.is_some());
    // Audit before the image leaves; a miss is an access too.
    if let Err(r) = audit_resource(
        &ctx.db,
        &agent,
        Access {
            action: Action::Read,
            outcome: if photo.is_some() { 0 } else { 4 },
            purpose,
            resource: "Patient/photo",
            patient_id: patient_id.as_deref().or(Some(&id)),
        },
        serde_json::json!({ "version": photo.as_ref().map(|p| p.version), "found": photo.is_some() }),
    )
    .await
    {
        return r.into_response();
    }
    match photo.as_ref().and_then(image_response) {
        Some(r) => r,
        None => store_failure(&StoreError::NotFound),
    }
}

async fn write(
    ctx: &AppContext,
    agent: &Agent,
    subject: &Subject,
    headers: &HeaderMap,
    id: &str,
    query: &VersionQuery,
    bytes: Option<&[u8]>,
) -> Response {
    let purpose = match purpose_of(headers) {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    if let Err(r) = authorize(
        &ctx.db,
        agent,
        subject,
        Resource::PatientIdentity,
        policy::Action::Update,
        purpose,
        Some(id),
    )
    .await
    {
        return r.into_response();
    }
    let Some(expected) = query.expected_version else {
        return ApiError::new(
            StatusCode::BAD_REQUEST,
            "give expected_version (0 when there is no photo yet)",
        )
        .into_response();
    };
    let tx = match begin(&ctx.db).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match store::put(&tx, id, expected, &agent.0, query.reason.as_deref(), bytes).await {
        Ok(photo) => {
            if let Err(r) = audit_resource(
                &tx,
                agent,
                Access {
                    action: Action::Update,
                    outcome: 0,
                    purpose,
                    resource: "Patient/photo",
                    patient_id: Some(&photo.patient_id),
                },
                serde_json::json!({
                    "version": photo.version,
                    "withdrawn": photo.bytes.is_none(),
                    "bytes": photo.bytes.as_ref().map(Vec::len),
                    "content_type": photo.content_type,
                }),
            )
            .await
            {
                return r.into_response();
            }
            if let Err(r) = commit(tx).await {
                return r.into_response();
            }
            axum::Json(serde_json::json!({
                "version": photo.version,
                "withdrawn": photo.bytes.is_none(),
            }))
            .into_response()
        }
        Err(e) => {
            let _ = sea_orm::TransactionSession::rollback(tx).await;
            let refused = matches!(
                e,
                StoreError::NotFound | StoreError::Conflict { .. } | StoreError::BadQuery(_)
            );
            if refused
                && let Err(r) = audit_resource(
                    &ctx.db,
                    agent,
                    Access {
                        action: Action::Update,
                        outcome: 4,
                        purpose,
                        resource: "Patient/photo",
                        patient_id: None,
                    },
                    // The refusal text names no image content.
                    serde_json::json!({ "refused": e.to_string() }),
                )
                .await
            {
                return r.into_response();
            }
            match e {
                // A refused file is the uploader's problem to fix, like a refused identity record.
                StoreError::BadQuery(m) => ApiError::invalid(vec![m.to_string()]).into_response(),
                other => store_failure(&other),
            }
        }
    }
}

pub(super) async fn put(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<VersionQuery>,
    body: Bytes,
) -> Response {
    write(&ctx, &agent, &subject, &headers, &id, &query, Some(&body)).await
}

pub(super) async fn withdraw(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<VersionQuery>,
) -> Response {
    write(&ctx, &agent, &subject, &headers, &id, &query, None).await
}

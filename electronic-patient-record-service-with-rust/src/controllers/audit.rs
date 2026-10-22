//! Audit trail endpoints (task AU-1 acceptance: a verify endpoint that detects a tampered row).
//!
//! `GET /api/audit/verify` recomputes the hash chain and reports whether it is intact. It is for
//! the `auditor` role. Verifying is itself an access to the audit log, so it writes its own audit
//! row (and answers 500 without a result if it cannot).
//!
//! `GET /api/audit` searches the trail (task AU-2), newest first, 50 per page. Auditor only. A
//! search is an access to the audit log, so it writes its own row naming which criteria were used
//! and how many rows came back, not their values.

use super::common::authorize;
use crate::api_error::ApiError;
use crate::audit::{self, Action, Chain, Event, Head, Verdict};
use crate::auth::Agent;
use crate::policy::{self, Purpose, Resource, Subject};
use axum::{
    Extension,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use sea_orm::DatabaseConnection;

/// What the chain can and cannot show, by whether it is keyed and whether a checkpoint was given.
fn assurance(keyed: bool, checkpointed: bool) -> String {
    let key = if keyed {
        "keyed: a row cannot be edited or removed and the chain recomputed without the key."
    } else {
        "unkeyed (EPR_INTEGRITY_MAC_KEY is not set): an edited row is detected, but someone who \
         can write to the database can recompute a consistent chain."
    };
    let tail = if checkpointed {
        "Rows up to the checkpoint you supplied are confirmed present; rows removed after it are \
         not detected until you publish a newer one."
    } else {
        "Removing the newest rows is not detected unless you supply a checkpoint published earlier \
         (`head` in a previous answer, stored where the database's owner cannot write)."
    };
    format!("{key} {tail}")
}

/// A checkpoint the caller published from an earlier answer's `head`.
#[derive(Debug, serde::Deserialize)]
pub struct CheckpointQuery {
    /// The head row's id.
    pub checkpoint_id: Option<i64>,
    /// The head row's digest.
    pub checkpoint_digest: Option<String>,
}

impl CheckpointQuery {
    /// Both parts or neither; one alone is a malformed request.
    ///
    /// # Errors
    ///
    /// When only one of the two parts is given.
    pub fn head(self) -> Result<Option<Head>, &'static str> {
        match (self.checkpoint_id, self.checkpoint_digest) {
            (None, None) => Ok(None),
            (Some(id), Some(digest)) => Ok(Some(Head { id, digest })),
            _ => Err("checkpoint_id and checkpoint_digest go together"),
        }
    }
}

/// Verify the chain and audit the verification, returning the HTTP status and body. A broken chain
/// is `409`, not `200`, so a monitor that only looks at the status still notices.
///
/// # Errors
///
/// When the rows cannot be read or the audit row cannot be written.
pub async fn verify_audited(
    db: &DatabaseConnection,
    chain: &Chain,
    agent: &Agent,
    checkpoint: Option<&Head>,
) -> Result<(StatusCode, serde_json::Value), audit::AuditError> {
    let (verdict, head) = audit::verify_against(db, chain, audit::VERIFY_BATCH, checkpoint).await?;
    let assurance = assurance(chain.keyed(), checkpoint.is_some());
    let (status, body) = match &verdict {
        Verdict::Intact { rows } => (
            StatusCode::OK,
            serde_json::json!({
                "intact": true,
                "rows": rows,
                "keyed": chain.keyed(),
                "head": head,
                "checkpoint_confirmed": checkpoint.is_some(),
                "assurance": assurance,
            }),
        ),
        Verdict::Unchecked {
            rows,
            unchecked,
            first_id,
            causes,
        } => (
            // Not 409: nothing was found wrong, so a monitor must not read this as tampering. Not
            // 200 either: the chain was not fully checked.
            StatusCode::FAILED_DEPENDENCY,
            serde_json::json!({
                "intact": false,
                "tampering_found": false,
                "error": "key-not-held",
                "rows": rows,
                "unchecked": unchecked,
                "first_unchecked_id": first_id,
                "causes": causes,
                "keyed": chain.keyed(),
                "head": head,
                "assurance": "No row was found altered, but the rows listed could not be checked because this service does not hold the key that made their digest. This is a key problem, not evidence of tampering: configure the missing key (EPR_INTEGRITY_MAC_KEY, or EPR_INTEGRITY_MAC_KEYS_RETIRED for an old one) or upgrade the service, then verify again.",
            }),
        ),
        Verdict::Broken { id, reason } => (
            StatusCode::CONFLICT,
            serde_json::json!({
                "intact": false,
                "first_broken_id": id,
                "reason": reason,
                "keyed": chain.keyed(),
                "assurance": assurance,
            }),
        ),
    };
    let mut event = Event::new(Action::Execute, agent.0.clone());
    event.resource_ref = Some("AuditLog/$verify".to_string());
    event.purpose_of_use = Some(Purpose::Audit.code().to_string());
    event.detail = Some(match &verdict {
        Verdict::Intact { rows } => serde_json::json!({ "intact": true, "rows": rows }),
        Verdict::Broken { id, .. } => serde_json::json!({ "intact": false, "first_broken_id": id }),
        Verdict::Unchecked {
            unchecked,
            first_id,
            ..
        } => serde_json::json!({
            "intact": false,
            "tampering_found": false,
            "unchecked": unchecked,
            "first_unchecked_id": first_id,
        }),
    });
    audit::append(db, chain, &event).await?;
    Ok((status, body))
}

/// Search the audit trail and audit the search. Shared by the JSON and FHIR endpoints, which only
/// differ in how the rows are shown.
pub(super) async fn search_audited(
    db: &DatabaseConnection,
    agent: &Agent,
    query: &audit::AuditQuery,
) -> Result<(Vec<audit::AuditRow>, Option<i64>), Result<String, audit::AuditError>> {
    let query = &query.normalized();
    let mut rows = audit::search(db, query).await.map_err(|e| match e {
        audit::SearchError::BadQuery(m) => Ok(m),
        audit::SearchError::Audit(a) => Err(a),
    })?;
    let page = usize::try_from(audit::SEARCH_LIMIT).unwrap_or(usize::MAX);
    let more = rows.len() > page;
    rows.truncate(page);
    let next = if more {
        rows.last().map(|r| r.id)
    } else {
        None
    };
    let mut event = Event::new(Action::Read, agent.0.clone());
    event.resource_ref = Some("AuditLog".to_string());
    event.purpose_of_use = Some(Purpose::Audit.code().to_string());
    event.detail = Some(serde_json::json!({ "search": query.used(), "results": rows.len() }));
    audit::append(db, audit::chain(), &event)
        .await
        .map_err(Err)?;
    Ok((rows, next))
}

async fn search(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    Query(query): Query<audit::AuditQuery>,
) -> Response {
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::AuditLog,
        policy::Action::Search,
        Purpose::Audit,
        None,
    )
    .await
    {
        return *r;
    }
    match search_audited(&ctx.db, &agent, &query).await {
        Ok((rows, next)) => {
            axum::Json(serde_json::json!({ "events": rows, "next": next })).into_response()
        }
        Err(Ok(message)) => {
            ApiError::new(StatusCode::BAD_REQUEST, message.to_string()).into_response()
        }
        Err(Err(e)) => {
            tracing::error!(error = %e, "audit search failed");
            ApiError::internal().into_response()
        }
    }
}

/// One CSV cell. Quoted when it holds a comma, quote or line break; a cell that starts with `=`,
/// `+`, `-`, `@`, a tab or a carriage return gets a leading apostrophe so a spreadsheet will not
/// run it as a formula (audit values such as an agent or a free-text reason can be chosen by an
/// attacker).
fn csv_cell(value: &str) -> String {
    let guarded = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    if guarded.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

const CSV_HEADER: &str = "id,at,action,outcome,agent,patient,resource,purpose,break_glass,network_address,detail,digest,prev_digest\n";

/// The rows as CSV, header first.
fn to_csv(rows: &[audit::AuditRow]) -> String {
    let mut out = String::from(CSV_HEADER);
    for r in rows {
        let cells = [
            r.id.to_string(),
            r.at.clone(),
            r.action.clone(),
            r.outcome.to_string(),
            r.agent.clone(),
            r.patient.clone().unwrap_or_default(),
            r.resource.clone().unwrap_or_default(),
            r.purpose.clone().unwrap_or_default(),
            r.break_glass.to_string(),
            r.network_address.clone().unwrap_or_default(),
            r.detail
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            r.digest.clone().unwrap_or_default(),
            r.prev_digest.clone().unwrap_or_default(),
        ];
        out.push_str(
            &cells
                .iter()
                .map(|c| csv_cell(c))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

/// The rows as newline-delimited JSON, one object per line.
fn to_ndjson(rows: &[audit::AuditRow]) -> String {
    rows.iter()
        .filter_map(|r| serde_json::to_string(r).ok())
        .map(|l| l + "\n")
        .collect()
}

/// Export the trail (task AU-2): the same criteria as the search, up to 10,000 rows, with the
/// digests so the chain can be re-verified elsewhere. The answer is a file; when there are more
/// rows the header `x-next-before` carries the `before` for the next file. The export is itself
/// audited (`AuditLog/$export`): the criteria names and how many rows left, not their values.
async fn export(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    Query(pairs): Query<Vec<(String, String)>>,
) -> Response {
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::AuditLog,
        policy::Action::Export,
        Purpose::Audit,
        None,
    )
    .await
    {
        return *r;
    }
    let criteria = match audit::AuditQuery::from_pairs(&pairs) {
        Ok(c) => c.normalized(),
        Err(m) => {
            return ApiError::new(StatusCode::BAD_REQUEST, m.to_string()).into_response();
        }
    };
    let format = pairs
        .iter()
        .find(|(k, _)| k == "format")
        .map_or("csv", |(_, v)| v.as_str());
    if !matches!(format, "csv" | "ndjson") {
        return ApiError::new(StatusCode::BAD_REQUEST, "format must be csv or ndjson")
            .into_response();
    }
    let mut rows = match audit::export(&ctx.db, &criteria).await {
        Ok(r) => r,
        Err(audit::SearchError::BadQuery(m)) => {
            return ApiError::new(StatusCode::BAD_REQUEST, m.to_string()).into_response();
        }
        Err(audit::SearchError::Audit(e)) => {
            tracing::error!(error = %e, "audit export failed");
            return ApiError::internal().into_response();
        }
    };
    let page = usize::try_from(audit::EXPORT_LIMIT).unwrap_or(usize::MAX);
    let more = rows.len() > page;
    rows.truncate(page);
    let next = if more {
        rows.last().map(|r| r.id)
    } else {
        None
    };
    let mut event = Event::new(Action::Read, agent.0.clone());
    event.resource_ref = Some("AuditLog/$export".to_string());
    event.purpose_of_use = Some(Purpose::Audit.code().to_string());
    event.detail = Some(serde_json::json!({
        "export": criteria.used(), "format": format, "rows": rows.len(), "truncated": more,
    }));
    if let Err(e) = audit::append(&ctx.db, audit::chain(), &event).await {
        tracing::error!(error = %e, "audit export could not be audited");
        return ApiError::internal().into_response();
    }
    let (content_type, body, ext) = if format == "csv" {
        ("text/csv; charset=utf-8", to_csv(&rows), "csv")
    } else {
        ("application/x-ndjson", to_ndjson(&rows), "ndjson")
    };
    let mut response = (StatusCode::OK, body).into_response();
    let h = response.headers_mut();
    h.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static(content_type),
    );
    h.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!("attachment; filename=\"audit-export.{ext}\""))
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment")),
    );
    h.insert(
        axum::http::header::HeaderName::from_static("x-content-type-options"),
        axum::http::HeaderValue::from_static("nosniff"),
    );
    if let Some(id) = next
        && let Ok(v) = axum::http::HeaderValue::from_str(&id.to_string())
    {
        h.insert(
            axum::http::header::HeaderName::from_static("x-next-before"),
            v,
        );
    }
    response
}

async fn verify(
    State(ctx): State<AppContext>,
    Extension(agent): Extension<Agent>,
    Extension(subject): Extension<Subject>,
    Query(query): Query<CheckpointQuery>,
) -> Response {
    let checkpoint = match query.head() {
        Ok(c) => c,
        Err(m) => return ApiError::new(StatusCode::BAD_REQUEST, m.to_string()).into_response(),
    };
    // Verifying the audit trail is, by its nature, for the purpose `audit`.
    if let Err(r) = authorize(
        &ctx.db,
        &agent,
        &subject,
        Resource::AuditLog,
        policy::Action::Read,
        Purpose::Audit,
        None,
    )
    .await
    {
        return *r;
    }
    match verify_audited(&ctx.db, audit::chain(), &agent, checkpoint.as_ref()).await {
        Ok((status, body)) => (status, axum::Json(body)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "audit verification failed");
            ApiError::internal().into_response()
        }
    }
}

/// Routes under `/api/audit`.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/audit")
        .add("/", get(search))
        .add("/export", get(export))
        .add("/verify", get(verify))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(agent: &str) -> audit::AuditRow {
        audit::AuditRow {
            id: 1,
            at: "2026-10-21T09:00:00.000000Z".into(),
            action: "R".into(),
            outcome: 0,
            agent: agent.into(),
            patient: None,
            resource: Some("Patient".into()),
            purpose: None,
            break_glass: false,
            network_address: None,
            detail: Some(serde_json::json!({"a": "x,y"})),
            digest: Some("sha256:ab".into()),
            prev_digest: None,
        }
    }

    #[test]
    fn a_cell_that_could_run_as_a_formula_is_defused_and_odd_characters_are_quoted() {
        for lead in ["=", "+", "-", "@"] {
            assert_eq!(csv_cell(&format!("{lead}cmd")), format!("'{lead}cmd"));
        }
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_cell("two\nlines"), "\"two\nlines\"");
        assert_eq!(csv_cell("=1,2"), "\"'=1,2\"");
    }

    #[test]
    fn csv_has_a_header_one_line_per_row_and_the_digests() {
        let csv = to_csv(&[row("=HYPERLINK(\"x\")"), row("sub-2")]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("id,at,action,outcome,agent,"));
        assert!(
            lines[1].contains("\"'=HYPERLINK(\"\"x\"\")\""),
            "{}",
            lines[1]
        );
        assert!(lines[2].ends_with(",sha256:ab,"), "{}", lines[2]);
        assert!(
            lines[2].contains("\"{\"\"a\"\":\"\"x,y\"\"}\""),
            "{}",
            lines[2]
        );
    }

    #[test]
    fn ndjson_is_one_json_object_per_line_with_the_digests() {
        let out = to_ndjson(&[row("a"), row("b")]);
        let lines: Vec<serde_json::Value> = out
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["digest"], "sha256:ab");
        assert!(lines[0].get("prev_digest").is_none());
    }

    #[test]
    fn export_criteria_read_numbers_and_booleans_from_a_query_string() {
        let pairs = |v: &[(&str, &str)]| -> Vec<(String, String)> {
            v.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect()
        };
        let q = audit::AuditQuery::from_pairs(&pairs(&[
            ("outcome", "0"),
            ("break_glass", "true"),
            ("before", "90"),
            ("action", "R"),
            ("after", "2026-10-01"),
            ("format", "ndjson"),
        ]))
        .unwrap();
        assert_eq!(
            (q.outcome, q.break_glass, q.before),
            (Some(0), Some(true), Some(90))
        );
        assert_eq!(
            (q.action.as_deref(), q.after.as_deref()),
            (Some("R"), Some("2026-10-01"))
        );
        // An empty form field is no criterion; a bad value names its key.
        let blank =
            audit::AuditQuery::from_pairs(&pairs(&[("outcome", ""), ("before", "")])).unwrap();
        assert_eq!((blank.outcome, blank.before), (None, None));
        for bad in [("outcome", "x"), ("break_glass", "maybe"), ("before", "x")] {
            let e = audit::AuditQuery::from_pairs(&pairs(&[bad])).unwrap_err();
            assert!(e.contains(bad.0), "{e}");
        }
    }
}

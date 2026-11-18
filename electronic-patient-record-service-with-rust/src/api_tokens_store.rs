//! Integration tokens (TL-1): issued once and kept only as a hash (`axa::api_tokens`), named, scoped, expiring and
//! revocable. Migration 41 (`api_tokens`, `api_token_versions`, `api_token_uses`).
//!
//! - **Shown once.** [`issue`] returns the whole token, to give the caller once; only its id and the SHA-256 hash of its
//!   secret are stored, and nothing here returns either again. A token reads `epr_<id>_<secret>`, so a secret scanner
//!   can find a leaked one.
//! - **Named.** A token's name is its audit agent (`token:<name>`) and is never reused, even after revocation.
//! - **Scoped.** A scope is `<resource>:<action>` (`patient-identity:create`) and must be one the integrator role holds
//!   ([`scope_words`]), so a token never reaches further than an integration already can; most hold one.
//! - **Expiring.** Every token expires, 1 to 365 days after it is issued.
//! - **Fixed.** Only the description changes; scopes, name and expiry never do. Revocation is final, with a reason.

use crate::patients_store::StoreError;
use crate::pg_time::PgTimestamp;
use crate::policy::{self, Action, Resource, Role};
use crate::pool::Db;
use axa::api_tokens::{self as tokens, Prefix, Record, Scope};
use jiff::{SignedDuration, Timestamp};
use serde::Serialize;
use sqlx::postgres::PgRow;
use sqlx::{PgConnection, Row};
use std::time::SystemTime;

crate::store_errors!("api_tokens");

/// Every token's prefix.
pub const PREFIX: &str = "epr";

/// The longest a token may live, in days.
pub const MAX_DAYS: i64 = 365;

/// The most scopes one token may hold (the table refuses more).
pub const MAX_SCOPES: usize = 16;

/// The longest description or revocation reason, in characters.
pub const MAX_TEXT: usize = 500;

/// The prefix, checked.
#[must_use]
pub fn prefix() -> Prefix {
    Prefix::new(PREFIX).expect("the prefix is valid")
}

/// A resource's name as a scope word: `PatientIdentity` is `patient-identity`.
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// The scope word for `action` on `resource`.
#[must_use]
pub fn scope_word(resource: Resource, action: Action) -> String {
    format!("{}:{}", kebab(resource.name()), action.name())
}

/// Every scope a token may hold: each `(resource, action)` the integrator role holds in the role table, in its order.
#[must_use]
pub fn scope_words() -> Vec<String> {
    Resource::ALL
        .into_iter()
        .flat_map(|r| {
            policy::permitted_actions(Role::Integrator, r)
                .iter()
                .map(move |a| scope_word(r, *a))
        })
        .collect()
}

/// A token as the list shows it: metadata only, never the token or its hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenView {
    /// The token's id: the middle part of the token, which is not secret.
    pub id: String,
    /// The version of its description and revocation; a change names it.
    pub version: i32,
    /// Its name, which is its audit agent.
    pub name: String,
    /// What it is for, in words.
    pub description: String,
    /// What it may do.
    pub scopes: Vec<String>,
    /// When it stops working (UTC).
    pub expires_at: String,
    /// Whether it has expired, at the time it was read.
    pub expired: bool,
    /// Whether it was revoked.
    pub revoked: bool,
    /// Why, when revoked.
    pub revoked_reason: Option<String>,
    /// Who issued it.
    pub created_by: String,
    /// When (UTC).
    pub created_at: String,
    /// When it was last used (UTC), at most a minute stale; absent if never.
    pub last_used_at: Option<String>,
}

/// A token just issued: the token, to show once, and its metadata.
#[derive(Debug, Clone, Serialize)]
pub struct IssuedView {
    /// The whole token. Shown once: it is not stored and cannot be read again.
    pub token: String,
    /// The token's metadata.
    #[serde(flatten)]
    pub view: TokenView,
}

/// What a request to issue a token asks for.
#[derive(Debug, Clone)]
pub struct NewToken<'a> {
    /// The name: 2 to 64 lowercase letters, digits and `-`, starting with a letter.
    pub name: &'a str,
    /// What it is for, at most 500 characters.
    pub description: &'a str,
    /// Its scopes: 1 to 16, each from [`scope_words`], none repeated.
    pub scopes: &'a [String],
    /// How many days it lives, 1 to 365.
    pub days: i64,
}

/// What is wrong with a request to issue a token, if anything.
#[must_use]
pub fn problems(new: &NewToken<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let name_ok = (2..=64).contains(&new.name.len())
        && new.name.starts_with(|c: char| c.is_ascii_lowercase())
        && new
            .name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !name_ok {
        out.push(
            "name must be 2 to 64 lowercase letters, digits and -, starting with a letter".into(),
        );
    }
    if new.description.chars().count() > MAX_TEXT {
        out.push(format!("description is longer than {MAX_TEXT} characters"));
    }
    if new.scopes.is_empty() || new.scopes.len() > MAX_SCOPES {
        out.push(format!("a token needs 1 to {MAX_SCOPES} scopes"));
    }
    let known = scope_words();
    for (i, s) in new.scopes.iter().enumerate() {
        if !known.contains(s) {
            out.push(format!(
                "scope {s} is not one an integration may hold (one of: {})",
                known.join(", ")
            ));
        } else if new.scopes[..i].contains(s) {
            out.push(format!("scope {s} is given twice"));
        }
    }
    if !(1..=MAX_DAYS).contains(&new.days) {
        out.push(format!("a token lives 1 to {MAX_DAYS} days"));
    }
    out
}

const COLUMNS: &str = "token_id::text AS id, version, name, description, scopes, \
     expires_at, revoked, reason, created_by, \
     to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS created_at, \
     (SELECT u.last_used_at FROM api_token_uses u WHERE u.token_id = c.token_id) AS last_used_at";

fn row_to_view(row: &PgRow, now: Timestamp) -> Result<TokenView, StoreError> {
    let expires: PgTimestamp = row.try_get("expires_at").map_err(db_err)?;
    let used: Option<PgTimestamp> = row.try_get("last_used_at").map_err(db_err)?;
    let id: String = row.try_get("id").map_err(db_err)?;
    Ok(TokenView {
        id: id.replace('-', ""),
        version: row.try_get("version").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        description: row.try_get("description").map_err(db_err)?,
        scopes: row.try_get("scopes").map_err(db_err)?,
        expires_at: crate::clock::rfc3339_micros(expires.0),
        expired: expires.0 <= now,
        revoked: row.try_get("revoked").map_err(db_err)?,
        revoked_reason: row.try_get("reason").map_err(db_err)?,
        created_by: row.try_get("created_by").map_err(db_err)?,
        created_at: row.try_get("created_at").map_err(db_err)?,
        last_used_at: used.map(|u| crate::clock::rfc3339_micros(u.0)),
    })
}

/// `t` as a [`SystemTime`], which [`Record::verify`] compares. A time before 1970 (never a token's) is the epoch.
fn system_time(t: Timestamp) -> SystemTime {
    let nanos = u64::try_from(t.as_nanosecond()).unwrap_or(0);
    SystemTime::UNIX_EPOCH + std::time::Duration::from_nanos(nanos)
}

/// A token's id as the database keeps it: the 32 hex digits as a UUID. `None` for anything else.
fn token_uuid(id: &str) -> Option<String> {
    (id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| uuid::Uuid::parse_str(id).ok())
        .flatten()
        .map(|u| u.hyphenated().to_string())
}

/// One token's metadata, current version.
///
/// # Errors
///
/// [`StoreError::NotFound`].
pub async fn one(db: impl Into<Db<'_>>, id: &str) -> Result<TokenView, StoreError> {
    let uuid = token_uuid(id).ok_or(StoreError::NotFound)?;
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM api_token_current c WHERE token_id = $1::uuid"
    )))
    .bind(uuid)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    .ok_or(StoreError::NotFound)?;
    row_to_view(&row, Timestamp::now())
}

/// Issue a token, in the caller's transaction: the token to show once, and its metadata. `by` is who issued it.
///
/// # Errors
///
/// [`StoreError::BadQuery`] for [`problems`]; [`StoreError::Conflict`] when the name is taken (names are never
/// reused); [`StoreError::Db`].
pub async fn issue(
    conn: &mut PgConnection,
    new: &NewToken<'_>,
    by: &str,
) -> Result<IssuedView, StoreError> {
    if let Some(first) = problems(new).into_iter().next() {
        return Err(StoreError::BadQuery(first));
    }
    for s in new.scopes {
        Scope::new(s).map_err(|e| StoreError::BadQuery(e.to_string()))?;
    }
    let taken: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM api_tokens WHERE name = $1)")
            .bind(new.name)
            .fetch_one(&mut *conn)
            .await
            .map_err(db_err)?;
    if taken {
        return Err(StoreError::Conflict { current: 0 });
    }
    let issued = tokens::issue(&prefix());
    let uuid = token_uuid(issued.id()).ok_or_else(|| StoreError::Db("issued a bad id".into()))?;
    let expires = Timestamp::now() + SignedDuration::from_hours(24 * new.days);
    sqlx::query(
        "INSERT INTO api_tokens (token_id, name, hash, scopes, expires_at, created_by)
         VALUES ($1::uuid, $2, $3, $4, $5, $6)",
    )
    .bind(&uuid)
    .bind(new.name)
    .bind(issued.hash())
    .bind(new.scopes)
    .bind(PgTimestamp(expires))
    .bind(by)
    .execute(&mut *conn)
    .await
    .map_err(db_err)?;
    sqlx::query(
        "INSERT INTO api_token_versions (token_id, version, description, revoked, reason, author)
         VALUES ($1::uuid, 1, $2, false, NULL, $3)",
    )
    .bind(&uuid)
    .bind(new.description.trim())
    .bind(by)
    .execute(&mut *conn)
    .await
    .map_err(db_err)?;
    let view = one(&mut *conn, issued.id()).await?;
    Ok(IssuedView {
        token: issued.token().to_string(),
        view,
    })
}

/// Every token's metadata, newest first.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn list(db: impl Into<Db<'_>>) -> Result<Vec<TokenView>, StoreError> {
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let now = Timestamp::now();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS} FROM api_token_current c ORDER BY created_at DESC, token_id"
    )))
    .fetch_all(&mut *conn)
    .await
    .map_err(db_err)?
    .iter()
    .map(|r| row_to_view(r, now))
    .collect()
}

/// Append the next version: a new description, or revocation with a reason. A revoked token takes no further version.
async fn append(
    conn: &mut PgConnection,
    id: &str,
    expected_version: i32,
    description: &str,
    reason: Option<&str>,
    by: &str,
) -> Result<TokenView, StoreError> {
    let was = one(&mut *conn, id).await?;
    if was.revoked {
        return Err(StoreError::BadQuery("the token is revoked".into()));
    }
    if was.version != expected_version {
        return Err(StoreError::Conflict {
            current: was.version,
        });
    }
    let uuid = token_uuid(id).ok_or(StoreError::NotFound)?;
    let appended = crate::store::append_version(
        &mut *conn,
        STORE,
        sqlx::query(
            "INSERT INTO api_token_versions (token_id, version, description, revoked, reason, author)
             VALUES ($1::uuid, $2, $3, $4, $5, $6) RETURNING token_id",
        )
        .bind(uuid)
        .bind(expected_version + 1)
        .bind(description)
        .bind(reason.is_some())
        .bind(reason)
        .bind(by),
    )
    .await?;
    if matches!(appended, crate::store::Appended::Taken) {
        let current = one(&mut *conn, id).await?.version;
        return Err(StoreError::Conflict { current });
    }
    one(&mut *conn, id).await
}

/// Change a token's description, at the version read.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (a stale version), [`StoreError::BadQuery`] (too long, or the
/// token is revoked).
pub async fn describe(
    conn: &mut PgConnection,
    id: &str,
    expected_version: i32,
    description: &str,
    by: &str,
) -> Result<TokenView, StoreError> {
    if description.chars().count() > MAX_TEXT {
        return Err(StoreError::BadQuery(format!(
            "description is longer than {MAX_TEXT} characters"
        )));
    }
    append(conn, id, expected_version, description.trim(), None, by).await
}

/// Revoke a token, at the version read, with a reason. Final: a revoked token is refused from then on and takes no
/// further change.
///
/// # Errors
///
/// [`StoreError::NotFound`], [`StoreError::Conflict`] (a stale version), [`StoreError::BadQuery`] (no reason, too
/// long, or already revoked).
pub async fn revoke(
    conn: &mut PgConnection,
    id: &str,
    expected_version: i32,
    reason: &str,
    by: &str,
) -> Result<TokenView, StoreError> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > MAX_TEXT {
        return Err(StoreError::BadQuery(format!(
            "a reason is needed, at most {MAX_TEXT} characters"
        )));
    }
    let was = one(&mut *conn, id).await?;
    append(
        conn,
        id,
        expected_version,
        &was.description,
        Some(reason),
        by,
    )
    .await
}

/// What a request needs to check a presented token: its record, for [`Record::verify`], and its name, for the audit
/// agent. `None` when there is no token with that id.
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn record(
    db: impl Into<Db<'_>>,
    id: &str,
) -> Result<Option<(Record, String)>, StoreError> {
    let Some(uuid) = token_uuid(id) else {
        return Ok(None);
    };
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    let Some(row) = sqlx::query(
        "SELECT c.hash, c.scopes, c.expires_at, c.revoked, c.name,
                (SELECT u.last_used_at FROM api_token_uses u WHERE u.token_id = c.token_id) AS last_used_at
         FROM api_token_current c WHERE c.token_id = $1::uuid",
    )
    .bind(uuid)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db_err)?
    else {
        return Ok(None);
    };
    let scopes: Vec<String> = row.try_get("scopes").map_err(db_err)?;
    let expires: PgTimestamp = row.try_get("expires_at").map_err(db_err)?;
    let used: Option<PgTimestamp> = row.try_get("last_used_at").map_err(db_err)?;
    let record = Record {
        id: id.to_string(),
        hash: row.try_get("hash").map_err(db_err)?,
        scopes: scopes.iter().filter_map(|s| Scope::new(s).ok()).collect(),
        expires_at: Some(system_time(expires.0)),
        revoked: row.try_get("revoked").map_err(db_err)?,
        last_used_at: used.map(|u| system_time(u.0)),
    };
    Ok(Some((record, row.try_get("name").map_err(db_err)?)))
}

/// Record that a token was used at `at` (at most once a minute per token: the caller asks [`axa::api_tokens::Verified::record_use`]).
///
/// # Errors
///
/// [`StoreError::Db`].
pub async fn record_use(db: impl Into<Db<'_>>, id: &str, at: Timestamp) -> Result<(), StoreError> {
    let uuid = token_uuid(id).ok_or(StoreError::NotFound)?;
    let mut conn = db.into().acquire().await.map_err(db_err)?;
    sqlx::query(
        "INSERT INTO api_token_uses (token_id, last_used_at) VALUES ($1::uuid, $2)
         ON CONFLICT (token_id) DO UPDATE SET last_used_at = EXCLUDED.last_used_at",
    )
    .bind(uuid)
    .bind(PgTimestamp(at))
    .execute(&mut *conn)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tokens_scopes_are_exactly_what_the_integrator_role_holds() {
        assert_eq!(
            scope_words(),
            [
                "patient-identity:search",
                "patient-identity:create",
                "patient-identity:update",
                "death-assertion:create",
                "referral-queue:create",
            ]
        );
        for s in scope_words() {
            assert!(Scope::new(&s).is_ok(), "{s} is a valid scope");
        }
        assert_eq!(
            scope_word(Resource::AuditLog, Action::Export),
            "audit-log:export"
        );
    }

    #[test]
    fn a_request_is_refused_for_a_bad_name_an_unknown_or_repeated_scope_or_a_life_out_of_range() {
        let one = vec!["referral-queue:create".to_string()];
        let ok = NewToken {
            name: "referral-feed",
            description: "The inbound referral feed",
            scopes: &one,
            days: 90,
        };
        assert!(problems(&ok).is_empty());
        let bad_names = ["a", "Referral", "9feed", "feed_one", &"x".repeat(65)];
        for name in bad_names {
            assert!(
                !problems(&NewToken { name, ..ok.clone() }).is_empty(),
                "{name}"
            );
        }
        let audit = vec!["audit-log:export".to_string()];
        assert!(
            problems(&NewToken {
                scopes: &audit,
                ..ok.clone()
            })[0]
                .contains("not one an integration")
        );
        let twice = vec![
            "referral-queue:create".to_string(),
            "referral-queue:create".to_string(),
        ];
        assert!(
            problems(&NewToken {
                scopes: &twice,
                ..ok.clone()
            })[0]
                .contains("twice")
        );
        let none: Vec<String> = vec![];
        assert!(
            !problems(&NewToken {
                scopes: &none,
                ..ok.clone()
            })
            .is_empty()
        );
        for days in [0, 366, -1] {
            assert!(
                !problems(&NewToken { days, ..ok.clone() }).is_empty(),
                "{days}"
            );
        }
        let long = "x".repeat(MAX_TEXT + 1);
        assert!(
            !problems(&NewToken {
                description: &long,
                ..ok
            })
            .is_empty()
        );
    }

    #[test]
    fn only_32_hex_digits_are_a_token_id() {
        let id = tokens::issue(&prefix()).id().to_string();
        assert!(token_uuid(&id).is_some());
        for bad in [
            "",
            "not-a-token",
            &id[..31],
            &format!("{id}0"),
            &id.replace('a', "g"),
        ] {
            assert!(token_uuid(bad).is_none() || bad == id, "{bad}");
        }
    }
}

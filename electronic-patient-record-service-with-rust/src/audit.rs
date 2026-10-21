//! The audit writer (plan task AU-1): one row per access, hash-chained.
//!
//! Each row's `digest` covers the previous row's `digest`, so removing,
//! reordering or editing any row breaks every digest after it. With a root key
//! configured (`EPR_INTEGRITY_MAC_KEY`, or a file named by `EPR_INTEGRITY_MAC_KEY_FILE`; 32+ bytes of hex) the digest is a keyed MAC
//! (`integrity-mac`), which someone holding only the database cannot recompute.
//! Without a key it is an unkeyed SHA-256, which detects accidents but not an
//! attacker who can write SQL and recompute it; [`Chain::keyed`] says which.
//!
//! The table itself refuses `UPDATE`, `DELETE` and `TRUNCATE` (see the
//! migration), so the chain is a second line of defence.

use integrity_mac::{KeyConfig, KeySet};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::sync::OnceLock;

/// MAC domain for the audit chain.
const DOMAIN: &str = "audit-chain";
/// Postgres advisory-lock key serialising chain appends ("EPRA" in ASCII).
const LOCK_KEY: i64 = 0x4550_5241;
/// How a timestamp is read and written: microsecond precision, UTC, round-trips exactly.
const TS_FORMAT: &str = "YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"";

/// FHIR `AuditEvent.action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Create.
    Create,
    /// Read.
    Read,
    /// Update.
    Update,
    /// Delete (never used for clinical data; kept for the vocabulary).
    Delete,
    /// Execute, such as a computation over supplied data.
    Execute,
}

impl Action {
    /// The single-letter FHIR code stored in the table.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Action::Create => "C",
            Action::Read => "R",
            Action::Update => "U",
            Action::Delete => "D",
            Action::Execute => "E",
        }
    }
}

/// One access to record.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// What was done.
    pub action: Action,
    /// 0 success, 4 minor failure, 8 serious failure, 12 major failure.
    pub outcome: i16,
    /// Who did it: the token `sub`, or `anonymous` when login is off.
    pub agent: String,
    /// The patient concerned, if any.
    pub patient_ref: Option<String>,
    /// The resource concerned, if any.
    pub resource_ref: Option<String>,
    /// Why (a purpose of use code), if known.
    pub purpose_of_use: Option<String>,
    /// Whether break-the-glass was used.
    pub break_glass: bool,
    /// The caller's network address, if known.
    pub network_address: Option<String>,
    /// Free structured detail. Never put clinical content here.
    pub detail: Option<serde_json::Value>,
}

impl Event {
    /// A successful event by `agent`, with every optional field empty.
    #[must_use]
    pub fn new(action: Action, agent: impl Into<String>) -> Self {
        Self {
            action,
            outcome: 0,
            agent: agent.into(),
            patient_ref: None,
            resource_ref: None,
            purpose_of_use: None,
            break_glass: false,
            network_address: None,
            detail: None,
        }
    }
}

/// Computes chain digests.
pub struct Chain {
    keys: Option<KeySet>,
}

impl Chain {
    /// A chain using `keys` when a key is configured, else unkeyed SHA-256.
    #[must_use]
    pub fn new(keys: KeySet) -> Self {
        Self {
            keys: keys.is_enabled().then_some(keys),
        }
    }

    /// An unkeyed chain.
    #[must_use]
    pub fn unkeyed() -> Self {
        Self { keys: None }
    }

    /// Whether digests are keyed MACs.
    #[must_use]
    pub fn keyed(&self) -> bool {
        self.keys.is_some()
    }

    /// The digest of `event` recorded at `recorded_at` after a row whose digest was `prev`.
    #[must_use]
    pub fn digest(&self, prev: Option<&str>, recorded_at: &str, event: &Event) -> String {
        let preimage = preimage(prev, recorded_at, event);
        match self.keys.as_ref().and_then(|k| k.tag(DOMAIN, &preimage)) {
            Some(tag) => tag,
            None => format!("sha256:{}", hex(&Sha256::digest(&preimage))),
        }
    }

    /// Whether `stored` is the digest of the given inputs (constant-time for MACs).
    #[must_use]
    pub fn matches(
        &self,
        stored: &str,
        prev: Option<&str>,
        recorded_at: &str,
        event: &Event,
    ) -> bool {
        let preimage = preimage(prev, recorded_at, event);
        if stored.starts_with("sha256:") {
            return !self.keyed()
                && stored == format!("sha256:{}", hex(&Sha256::digest(&preimage)));
        }
        self.keys.as_ref().is_some_and(|k| {
            k.verify(DOMAIN, Some(stored), &preimage) == integrity_mac::MacVerdict::Valid
        })
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Length-prefixed fields, so no value can be read as part of another.
fn preimage(prev: Option<&str>, recorded_at: &str, e: &Event) -> Vec<u8> {
    let mut out = String::from("epr-audit/1;");
    let mut field = |name: &str, value: &str| {
        let _ = write!(out, "{name}={}:{value};", value.len());
    };
    field("prev", prev.unwrap_or(""));
    field("at", recorded_at);
    field("action", e.action.code());
    field("outcome", &e.outcome.to_string());
    field("agent", &e.agent);
    field("patient", e.patient_ref.as_deref().unwrap_or(""));
    field("resource", e.resource_ref.as_deref().unwrap_or(""));
    field("purpose", e.purpose_of_use.as_deref().unwrap_or(""));
    field("break_glass", if e.break_glass { "1" } else { "0" });
    field("addr", e.network_address.as_deref().unwrap_or(""));
    // serde_json orders object keys, so the text is stable across a JSONB round trip.
    field(
        "detail",
        &e.detail
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
    );
    out.into_bytes()
}

/// Key configuration: the variables are `EPR_INTEGRITY_MAC_KEY`, `EPR_INTEGRITY_MAC_KEY_FILE`,
/// `EPR_INTEGRITY_MAC_KEY_ID` and `EPR_INTEGRITY_MAC_KEYS_RETIRED` (derived by the crate).
const KEY_CONFIG: KeyConfig = KeyConfig::new("electronic-patient-record", "EPR");

static CHAIN: OnceLock<Chain> = OnceLock::new();

/// The process-wide chain, configured from the environment (see [`KEY_CONFIG`]). A key file that
/// cannot be read leaves the chain unkeyed and logs why; it never falls back to a stale inline key.
pub fn chain() -> &'static Chain {
    CHAIN.get_or_init(|| {
        let source = integrity_mac::resolve_root_key(
            std::env::var(KEY_CONFIG.key_env()).ok().as_deref(),
            std::env::var(KEY_CONFIG.key_file_env()).ok().as_deref(),
        );
        if let Some(w) = &source.warning {
            tracing::warn!("{w}");
        }
        if let Some(e) = &source.error {
            tracing::error!("{e}");
        }
        let retired = std::env::var(KEY_CONFIG.retired_keys_env()).ok();
        let chain = Chain::new(KeySet::load(&KEY_CONFIG, source.key.as_deref(), retired.as_deref()));
        if !chain.keyed() {
            tracing::warn!(
                "audit chain is UNKEYED (no usable {}): it detects accidents, not someone who can write SQL",
                KEY_CONFIG.key_env()
            );
        }
        chain
    })
}

fn pg(sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}

/// Why an audit operation failed.
#[derive(Debug)]
pub struct AuditError(pub String);

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "audit failed: {}", self.0)
    }
}

impl std::error::Error for AuditError {}

fn err(e: impl std::fmt::Display) -> AuditError {
    AuditError(e.to_string())
}

/// Append `event` to the chain, in its own transaction, and return the new row's id.
///
/// Appends are serialised by a transaction-scoped advisory lock, so two
/// concurrent callers cannot both extend the same predecessor.
///
/// # Errors
///
/// When the database refuses. Callers must treat that as a failure of the whole
/// request: an access that cannot be audited does not happen.
pub async fn append(
    db: &DatabaseConnection,
    chain: &Chain,
    event: &Event,
) -> Result<i64, AuditError> {
    let tx = db.begin().await.map_err(err)?;
    tx.execute_raw(pg(
        "SELECT pg_advisory_xact_lock($1)",
        vec![LOCK_KEY.into()],
    ))
    .await
    .map_err(err)?;
    let prev = tx
        .query_one_raw(pg(
            "SELECT digest FROM audit_events ORDER BY id DESC LIMIT 1",
            vec![],
        ))
        .await
        .map_err(err)?
        .map(|r| r.try_get::<String>("", "digest"))
        .transpose()
        .map_err(err)?;
    let at: String = tx
        .query_one_raw(pg(
            &format!("SELECT to_char(clock_timestamp() AT TIME ZONE 'UTC', '{TS_FORMAT}') AS ts"),
            vec![],
        ))
        .await
        .map_err(err)?
        .ok_or_else(|| AuditError("no timestamp".into()))?
        .try_get("", "ts")
        .map_err(err)?;
    let digest = chain.digest(prev.as_deref(), &at, event);
    let row = tx
        .query_one_raw(pg(
            &format!(
                "INSERT INTO audit_events
                   (recorded_at, action, outcome, agent, patient_ref, resource_ref, purpose_of_use,
                    break_glass, network_address, detail, prev_digest, digest)
                 VALUES (to_timestamp($1, '{TS_FORMAT}') AT TIME ZONE 'UTC' AT TIME ZONE 'UTC',
                         $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                 RETURNING id"
            ),
            vec![
                at.into(),
                event.action.code().into(),
                event.outcome.into(),
                event.agent.clone().into(),
                event.patient_ref.clone().into(),
                event.resource_ref.clone().into(),
                event.purpose_of_use.clone().into(),
                event.break_glass.into(),
                event.network_address.clone().into(),
                event.detail.clone().into(),
                prev.into(),
                digest.into(),
            ],
        ))
        .await
        .map_err(err)?
        .ok_or_else(|| AuditError("insert returned no row".into()))?;
    let id: i64 = row.try_get("", "id").map_err(err)?;
    tx.commit().await.map_err(err)?;
    Ok(id)
}

/// The result of verifying the whole chain.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every row's digest is correct and links to its predecessor.
    Intact {
        /// How many rows were checked.
        rows: usize,
    },
    /// The first row where the chain breaks.
    Broken {
        /// The id of that row.
        id: i64,
        /// What is wrong with it.
        reason: String,
    },
}

/// One audit row about a patient, with only the fields a patient may be shown. `detail`,
/// `network_address` and the digests are left out on purpose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatientLogRow {
    /// The row id; newest rows have the largest.
    pub id: i64,
    /// When, as `YYYY-MM-DDTHH:MM:SS.ffffffZ` (UTC).
    pub at: String,
    /// The FHIR action code: `C`, `R`, `U`, `D` or `E`.
    pub action: String,
    /// 0 success, 4 and above a refusal or failure.
    pub outcome: i16,
    /// Who did it: the token `sub`. The caller decides how much of this to reveal.
    pub agent: String,
    /// What was accessed, as recorded (`Patient`, `Consent`, `PatientIdentity/Read`, …).
    pub resource: Option<String>,
    /// The declared purpose of use.
    pub purpose: Option<String>,
    /// Whether the access was a break-the-glass override.
    pub break_glass: bool,
}

/// The newest audit rows naming `Patient/<patient_id>`, newest first, at most `limit`, older than
/// `before` when given.
///
/// # Errors
///
/// When the rows cannot be read.
pub async fn patient_log(
    db: &DatabaseConnection,
    patient_id: &str,
    before: Option<i64>,
    limit: i64,
) -> Result<Vec<PatientLogRow>, AuditError> {
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT id, to_char(recorded_at AT TIME ZONE 'UTC', '{TS_FORMAT}') AS ts,
                        action::text AS action, outcome, agent, resource_ref, purpose_of_use, break_glass
                 FROM audit_events
                 WHERE lower(patient_ref) = lower($1) AND ($2::bigint IS NULL OR id < $2)
                 ORDER BY id DESC LIMIT $3"
            ),
            vec![
                format!("Patient/{}", patient_id.to_lowercase()).into(),
                before.into(),
                limit.into(),
            ],
        ))
        .await
        .map_err(err)?;
    rows.iter()
        .map(|r| {
            Ok(PatientLogRow {
                id: r.try_get("", "id").map_err(err)?,
                at: r.try_get("", "ts").map_err(err)?,
                action: r.try_get("", "action").map_err(err)?,
                outcome: r.try_get("", "outcome").map_err(err)?,
                agent: r.try_get("", "agent").map_err(err)?,
                resource: r.try_get("", "resource_ref").map_err(err)?,
                purpose: r.try_get("", "purpose_of_use").map_err(err)?,
                break_glass: r.try_get("", "break_glass").map_err(err)?,
            })
        })
        .collect()
}

/// A row's id and digest: the chain's head at some moment. Published somewhere the database's
/// owner cannot write, it is what lets a later check notice that the newest rows were removed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Head {
    /// The row's id.
    pub id: i64,
    /// The row's stored digest.
    pub digest: String,
}

/// Rows read per query by [`verify`], so memory does not grow with the table.
pub const VERIFY_BATCH: i64 = 1000;

/// Recompute every digest in id order.
///
/// The table is read in batches of [`VERIFY_BATCH`] by keyset, so a long chain is not held in
/// memory. A chain can only show that no row was edited or removed *before the last row*: the
/// removal of the newest rows leaves a shorter, still-consistent chain. Detecting that needs the
/// latest digest recorded somewhere an attacker with database access cannot write.
///
/// # Errors
///
/// When the rows cannot be read.
pub async fn verify(db: &DatabaseConnection, chain: &Chain) -> Result<Verdict, AuditError> {
    verify_in_batches(db, chain, VERIFY_BATCH).await
}

/// [`verify`] with a chosen batch size (the tests use a small one to cross batch boundaries).
///
/// # Errors
///
/// When the rows cannot be read.
pub async fn verify_in_batches(
    db: &DatabaseConnection,
    chain: &Chain,
    batch: i64,
) -> Result<Verdict, AuditError> {
    Ok(verify_against(db, chain, batch, None).await?.0)
}

/// [`verify_in_batches`] that also returns the chain's head and, given a `checkpoint` published
/// earlier, checks that the row it names still exists with the same digest. A missing row means
/// newer rows were removed (or the checkpoint is from another database); a different digest means
/// the row was rewritten. Both are [`Verdict::Broken`] at the checkpoint's id.
///
/// The head is the newest row *checked*, so it never includes the row a caller appends to record
/// the verification itself.
///
/// # Errors
///
/// When the rows cannot be read.
pub async fn verify_against(
    db: &DatabaseConnection,
    chain: &Chain,
    batch: i64,
    checkpoint: Option<&Head>,
) -> Result<(Verdict, Option<Head>), AuditError> {
    let missing = |c: &Head| Verdict::Broken {
        id: c.id,
        reason: format!(
            "the checkpoint row {} is missing: newer rows were removed, or the checkpoint is from another database",
            c.id
        ),
    };
    let mut head: Option<Head> = None;
    let mut previous: Option<String> = None;
    let mut last_id: i64 = 0;
    let mut checked: usize = 0;
    loop {
        let rows = db
            .query_all_raw(pg(
                &format!(
                    "SELECT id, to_char(recorded_at AT TIME ZONE 'UTC', '{TS_FORMAT}') AS ts, action::text AS action,
                            outcome, agent, patient_ref, resource_ref, purpose_of_use, break_glass,
                            network_address, detail, prev_digest, digest
                     FROM audit_events WHERE id > $1 ORDER BY id LIMIT $2"
                ),
                vec![last_id.into(), batch.into()],
            ))
            .await
            .map_err(err)?;
        if rows.is_empty() {
            if let Some(c) = checkpoint
                && head.as_ref().is_none_or(|h| h.id < c.id)
            {
                return Ok((missing(c), head));
            }
            return Ok((Verdict::Intact { rows: checked }, head));
        }
        for r in &rows {
            checked += 1;
            let id: i64 = r.try_get("", "id").map_err(err)?;
            last_id = id;
            if let Some(c) = checkpoint
                && id > c.id
                && head.as_ref().is_none_or(|h| h.id < c.id)
            {
                return Ok((missing(c), head));
            }
            let stored_prev: Option<String> = r.try_get("", "prev_digest").map_err(err)?;
            let digest: String = r.try_get("", "digest").map_err(err)?;
            if stored_prev != previous {
                return Ok((
                    Verdict::Broken {
                        id,
                        reason: "does not link to the previous row".into(),
                    },
                    head,
                ));
            }
            let action = match r.try_get::<String>("", "action").map_err(err)?.as_str() {
                "C" => Action::Create,
                "R" => Action::Read,
                "U" => Action::Update,
                "D" => Action::Delete,
                _ => Action::Execute,
            };
            let event = Event {
                action,
                outcome: r.try_get("", "outcome").map_err(err)?,
                agent: r.try_get("", "agent").map_err(err)?,
                patient_ref: r.try_get("", "patient_ref").map_err(err)?,
                resource_ref: r.try_get("", "resource_ref").map_err(err)?,
                purpose_of_use: r.try_get("", "purpose_of_use").map_err(err)?,
                break_glass: r.try_get("", "break_glass").map_err(err)?,
                network_address: r.try_get("", "network_address").map_err(err)?,
                detail: r.try_get("", "detail").map_err(err)?,
            };
            let ts: String = r.try_get("", "ts").map_err(err)?;
            if !chain.matches(&digest, previous.as_deref(), &ts, &event) {
                return Ok((
                    Verdict::Broken {
                        id,
                        reason: format!("digest of row {checked} does not match its content"),
                    },
                    head,
                ));
            }
            if let Some(c) = checkpoint
                && c.id == id
                && c.digest != digest
            {
                return Ok((
                    Verdict::Broken {
                        id,
                        reason: "does not match the published checkpoint".into(),
                    },
                    head,
                ));
            }
            head = Some(Head {
                id,
                digest: digest.clone(),
            });
            previous = Some(digest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    fn keyed() -> Chain {
        Chain::new(KeySet::load(&KeyConfig::new("t", "T"), Some(KEY), None))
    }

    fn ev() -> Event {
        let mut e = Event::new(Action::Read, "user-1");
        e.patient_ref = Some("Patient/1".into());
        e
    }

    #[test]
    fn digest_is_deterministic_and_covers_every_input() {
        let c = Chain::unkeyed();
        let base = c.digest(None, "2026-01-01T00:00:00.000000Z", &ev());
        assert_eq!(base, c.digest(None, "2026-01-01T00:00:00.000000Z", &ev()));
        assert_ne!(
            base,
            c.digest(Some("x"), "2026-01-01T00:00:00.000000Z", &ev())
        );
        assert_ne!(base, c.digest(None, "2026-01-01T00:00:00.000001Z", &ev()));
        let mut e = ev();
        e.agent = "user-2".into();
        assert_ne!(base, c.digest(None, "2026-01-01T00:00:00.000000Z", &e));
        let mut e = ev();
        e.break_glass = true;
        assert_ne!(base, c.digest(None, "2026-01-01T00:00:00.000000Z", &e));
        let mut e = ev();
        e.detail = Some(serde_json::json!({"k": 1}));
        assert_ne!(base, c.digest(None, "2026-01-01T00:00:00.000000Z", &e));
    }

    #[test]
    fn fields_cannot_bleed_into_each_other() {
        let c = Chain::unkeyed();
        let mut a = Event::new(Action::Read, "ab");
        a.patient_ref = Some("c".into());
        let mut b = Event::new(Action::Read, "a");
        b.patient_ref = Some("bc".into());
        assert_ne!(c.digest(None, "t", &a), c.digest(None, "t", &b));
    }

    #[test]
    fn keyed_digest_is_a_mac_and_verifies() {
        let c = keyed();
        assert!(c.keyed());
        let d = c.digest(None, "t", &ev());
        assert!(d.starts_with("d1.k1:"), "{d}");
        assert!(c.matches(&d, None, "t", &ev()));
        let mut other = ev();
        other.agent = "someone-else".into();
        assert!(!c.matches(&d, None, "t", &other));
    }

    #[test]
    fn an_unkeyed_digest_does_not_pass_as_keyed() {
        let unkeyed = Chain::unkeyed().digest(None, "t", &ev());
        assert!(!keyed().matches(&unkeyed, None, "t", &ev()));
    }

    #[test]
    fn a_key_that_is_too_short_leaves_the_chain_unkeyed() {
        let c = Chain::new(KeySet::load(&KeyConfig::new("t", "T"), Some("abcd"), None));
        assert!(!c.keyed());
    }

    #[test]
    fn the_key_variables_are_the_documented_ones() {
        assert_eq!(KEY_CONFIG.key_env(), "EPR_INTEGRITY_MAC_KEY");
        assert_eq!(KEY_CONFIG.key_file_env(), "EPR_INTEGRITY_MAC_KEY_FILE");
    }
}

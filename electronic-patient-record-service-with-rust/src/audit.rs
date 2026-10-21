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
        self.check(stored, prev, recorded_at, event) == RowCheck::Valid
    }

    /// Check `stored` against the given inputs, telling "the content does not match" apart from
    /// "this service cannot check it": a row MACed under a key (or by a scheme) that this process
    /// does not hold is neither confirmed nor refuted, and is not evidence of tampering.
    ///
    /// A `sha256:` row in a keyed chain is a [`RowCheck::Mismatch`]: someone without the key can
    /// write one, so accepting it would defeat the key. That also means rows written before a key
    /// was configured read as broken; see the audit spec.
    #[must_use]
    pub fn check(
        &self,
        stored: &str,
        prev: Option<&str>,
        recorded_at: &str,
        event: &Event,
    ) -> RowCheck {
        let preimage = preimage(prev, recorded_at, event);
        if stored.starts_with("sha256:") {
            let same = stored == format!("sha256:{}", hex(&Sha256::digest(&preimage)));
            return if !self.keyed() && same {
                RowCheck::Valid
            } else {
                RowCheck::Mismatch
            };
        }
        match &self.keys {
            Some(k) => match k.verify(DOMAIN, Some(stored), &preimage) {
                integrity_mac::MacVerdict::Valid => RowCheck::Valid,
                integrity_mac::MacVerdict::UnknownKey(id) if !id.is_empty() => {
                    RowCheck::KeyNotHeld(id)
                }
                integrity_mac::MacVerdict::UnknownScheme(s) => RowCheck::SchemeUnknown(s),
                _ => RowCheck::Mismatch,
            },
            // No key at all: a well-formed `[<scheme>.]<id>:<hex>` names a key we do not hold.
            None => match stored
                .split_once(':')
                .filter(|(p, h)| !p.is_empty() && !h.is_empty())
            {
                Some((prefix, _)) => match prefix.split_once('.') {
                    Some((scheme, _)) if scheme != integrity_mac::SCHEME => {
                        RowCheck::SchemeUnknown(scheme.to_string())
                    }
                    Some((_, id)) => RowCheck::KeyNotHeld(id.to_string()),
                    None => RowCheck::KeyNotHeld(prefix.to_string()),
                },
                None => RowCheck::Mismatch,
            },
        }
    }
}

/// What checking one row's digest found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowCheck {
    /// The digest is right.
    Valid,
    /// The digest is wrong: the row's content changed (or the digest was written without the key).
    Mismatch,
    /// The digest was made with a key this process does not hold. Not a mismatch: it can be neither
    /// confirmed nor refuted. The fix is the key (`EPR_INTEGRITY_MAC_KEY`, or the old one in
    /// `EPR_INTEGRITY_MAC_KEYS_RETIRED`), not an investigation of the data.
    KeyNotHeld(String),
    /// The digest names a key-derivation scheme this build does not implement: the binary is older
    /// than the row. The fix is to upgrade it.
    SchemeUnknown(String),
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

/// Who the current request's caller is, for the audit trail: their roles and organisations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Actor {
    /// Role codes, sorted.
    pub roles: Vec<String>,
    /// Organisation tokens, sorted.
    pub organisations: Vec<String>,
}

tokio::task_local! {
    /// Set by the authorization middleware around each request, so every audit row written while
    /// handling it carries the caller's roles and organisations without each call site passing
    /// them. Rows written outside a request (tests, jobs) carry no `actor`.
    pub static ACTOR: Actor;
}

/// `event`'s detail with the current request's [`ACTOR`] added under `"actor"`. An existing
/// `actor` key is overwritten: the trail records what the token said, not what a handler wrote.
fn with_actor(event: &Event) -> Event {
    let mut event = event.clone();
    let Ok(actor) = ACTOR.try_with(Clone::clone) else {
        return event;
    };
    let value = serde_json::json!({ "roles": actor.roles, "organisations": actor.organisations });
    match event.detail.take() {
        None => event.detail = Some(serde_json::json!({ "actor": value })),
        Some(serde_json::Value::Object(mut m)) => {
            m.insert("actor".into(), value);
            event.detail = Some(serde_json::Value::Object(m));
        }
        other => event.detail = other,
    }
    event
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
    let event = &with_actor(event);
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
    /// No row is wrong, but some could not be checked because this process does not hold the key
    /// (or does not know the scheme) that made their digest. This is a **key problem, not
    /// evidence of tampering**; every row that could be checked, and every link, was fine.
    Unchecked {
        /// Rows read.
        rows: usize,
        /// How many of them could not be checked.
        unchecked: usize,
        /// The first row that could not be checked.
        first_id: i64,
        /// Why, once per cause, sorted: e.g. `key k1 is not held`.
        causes: Vec<String>,
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
    /// The caller's roles when the row was written. Empty for rows from before roles were
    /// recorded, and for rows written outside a request.
    pub roles: Vec<String>,
    /// The caller's organisations, as for `roles`.
    pub organisations: Vec<String>,
}

/// The string values of `actor[key]`, or none if it is absent or not a list of strings.
fn strings(actor: &serde_json::Value, key: &str) -> Vec<String> {
    actor
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
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
                        action::text AS action, outcome, agent, resource_ref, purpose_of_use, break_glass,
                        detail->'actor' AS actor
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
            let actor: Option<serde_json::Value> = r.try_get("", "actor").map_err(err)?;
            let actor = actor.unwrap_or_default();
            Ok(PatientLogRow {
                id: r.try_get("", "id").map_err(err)?,
                at: r.try_get("", "ts").map_err(err)?,
                action: r.try_get("", "action").map_err(err)?,
                outcome: r.try_get("", "outcome").map_err(err)?,
                agent: r.try_get("", "agent").map_err(err)?,
                resource: r.try_get("", "resource_ref").map_err(err)?,
                purpose: r.try_get("", "purpose_of_use").map_err(err)?,
                break_glass: r.try_get("", "break_glass").map_err(err)?,
                roles: strings(&actor, "roles"),
                organisations: strings(&actor, "organisations"),
            })
        })
        .collect()
}

/// Criteria for [`search`]. Every field narrows; none set returns the newest rows.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct AuditQuery {
    /// Who did it: the token `sub`, exactly.
    pub agent: Option<String>,
    /// The patient: a bare id, matched case-insensitively against `Patient/<id>`.
    pub patient: Option<String>,
    /// `C`, `R`, `U`, `D` or `E`.
    pub action: Option<String>,
    /// 0, 4, 8 or 12.
    pub outcome: Option<i16>,
    /// A purpose-of-use code, exactly.
    pub purpose: Option<String>,
    /// Only break-the-glass rows (`true`) or only the others (`false`).
    pub break_glass: Option<bool>,
    /// Not before this instant: `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SSZ`, inclusive.
    pub from: Option<String>,
    /// Before this instant, same formats, exclusive.
    pub to: Option<String>,
    /// Rows older than this id (the previous page's `next`).
    pub before: Option<i64>,
}

impl AuditQuery {
    /// The same query with blank text criteria removed, so `?agent=` (an empty form field) means
    /// "no criterion" rather than "no row".
    #[must_use]
    pub fn normalized(&self) -> AuditQuery {
        let keep = |v: &Option<String>| v.clone().filter(|s| !s.trim().is_empty());
        AuditQuery {
            agent: keep(&self.agent),
            patient: keep(&self.patient),
            action: keep(&self.action),
            outcome: self.outcome,
            purpose: keep(&self.purpose),
            break_glass: self.break_glass,
            from: keep(&self.from),
            to: keep(&self.to),
            before: self.before,
        }
    }

    /// The names of the criteria that were set, for the audit row of the search itself (the values
    /// are identifiers and stay out of it).
    #[must_use]
    pub fn used(&self) -> Vec<&'static str> {
        [
            ("agent", self.agent.is_some()),
            ("patient", self.patient.is_some()),
            ("action", self.action.is_some()),
            ("outcome", self.outcome.is_some()),
            ("purpose", self.purpose.is_some()),
            ("break_glass", self.break_glass.is_some()),
            ("from", self.from.is_some()),
            ("to", self.to.is_some()),
            ("before", self.before.is_some()),
        ]
        .into_iter()
        .filter_map(|(k, set)| set.then_some(k))
        .collect()
    }
}

/// One audit row as an auditor sees it. The digests are left out: use the verify endpoint.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AuditRow {
    /// Row id.
    pub id: i64,
    /// When, UTC with microseconds.
    pub at: String,
    /// `C`, `R`, `U`, `D` or `E`.
    pub action: String,
    /// 0, 4, 8 or 12.
    pub outcome: i16,
    /// The token `sub`.
    pub agent: String,
    /// `Patient/<id>` when the access named one.
    pub patient: Option<String>,
    /// What was accessed.
    pub resource: Option<String>,
    /// The declared purpose of use.
    pub purpose: Option<String>,
    /// Whether break-the-glass was used.
    pub break_glass: bool,
    /// Caller's address, when recorded.
    pub network_address: Option<String>,
    /// Structured detail; never holds demographics or consent content by rule.
    pub detail: Option<serde_json::Value>,
}

/// Why a search failed.
#[derive(Debug)]
pub enum SearchError {
    /// The criteria were not understood; the message says which.
    BadQuery(String),
    /// The rows could not be read.
    Audit(AuditError),
}

/// The most rows [`search`] returns.
pub const SEARCH_LIMIT: i64 = 50;

/// Newest-first rows matching `q`, at most [`SEARCH_LIMIT`] + 1 (the extra row tells the caller
/// there is another page).
///
/// # Errors
///
/// [`SearchError::BadQuery`] for an unknown action, outcome or date; [`SearchError::Audit`] when
/// the rows cannot be read.
pub async fn search(db: &DatabaseConnection, q: &AuditQuery) -> Result<Vec<AuditRow>, SearchError> {
    let q = &q.normalized();
    if let Some(a) = &q.action
        && !matches!(a.as_str(), "C" | "R" | "U" | "D" | "E")
    {
        return Err(SearchError::BadQuery(
            "action must be one of C, R, U, D, E".into(),
        ));
    }
    if let Some(o) = q.outcome
        && !matches!(o, 0 | 4 | 8 | 12)
    {
        return Err(SearchError::BadQuery(
            "outcome must be 0, 4, 8 or 12".into(),
        ));
    }
    for (name, v) in [("from", &q.from), ("to", &q.to)] {
        if let Some(v) = v {
            let shaped = v.len() >= 10
                && v.is_char_boundary(10)
                && v[..10].bytes().enumerate().all(|(i, b)| {
                    if i == 4 || i == 7 {
                        b == b'-'
                    } else {
                        b.is_ascii_digit()
                    }
                })
                && (v.len() == 10 || (v.as_bytes()[10] == b'T' && v.ends_with('Z')));
            let valid = shaped
                && db
                    .query_one_raw(pg("SELECT $1::timestamptz AS t", vec![v.clone().into()]))
                    .await
                    .is_ok();
            if !valid {
                return Err(SearchError::BadQuery(format!(
                    "{name} must be YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ"
                )));
            }
        }
    }
    let rows = db
        .query_all_raw(pg(
            &format!(
                "SELECT id, to_char(recorded_at AT TIME ZONE 'UTC', '{TS_FORMAT}') AS ts,
                        action::text AS action, outcome, agent, patient_ref, resource_ref,
                        purpose_of_use, break_glass, network_address, detail
                 FROM audit_events
                 WHERE ($1::text IS NULL OR agent = $1)
                   AND ($2::text IS NULL OR lower(patient_ref) = 'patient/' || lower($2))
                   AND ($3::text IS NULL OR action::text = $3)
                   AND ($4::smallint IS NULL OR outcome = $4)
                   AND ($5::text IS NULL OR purpose_of_use = $5)
                   AND ($6::boolean IS NULL OR break_glass = $6)
                   AND ($7::text IS NULL OR recorded_at >= $7::timestamptz)
                   AND ($8::text IS NULL OR recorded_at < $8::timestamptz)
                   AND ($9::bigint IS NULL OR id < $9)
                 ORDER BY id DESC LIMIT $10"
            ),
            vec![
                q.agent.clone().into(),
                q.patient.clone().into(),
                q.action.clone().into(),
                q.outcome.into(),
                q.purpose.clone().into(),
                q.break_glass.into(),
                q.from.clone().into(),
                q.to.clone().into(),
                q.before.into(),
                (SEARCH_LIMIT + 1).into(),
            ],
        ))
        .await
        .map_err(|e| SearchError::Audit(err(e)))?;
    rows.iter()
        .map(|r| {
            let e = |x| SearchError::Audit(err(x));
            Ok(AuditRow {
                id: r.try_get("", "id").map_err(e)?,
                at: r.try_get("", "ts").map_err(e)?,
                action: r.try_get("", "action").map_err(e)?,
                outcome: r.try_get("", "outcome").map_err(e)?,
                agent: r.try_get("", "agent").map_err(e)?,
                patient: r.try_get("", "patient_ref").map_err(e)?,
                resource: r.try_get("", "resource_ref").map_err(e)?,
                purpose: r.try_get("", "purpose_of_use").map_err(e)?,
                break_glass: r.try_get("", "break_glass").map_err(e)?,
                network_address: r.try_get("", "network_address").map_err(e)?,
                detail: r.try_get("", "detail").map_err(e)?,
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
    let mut unchecked: usize = 0;
    let mut first_unchecked: Option<i64> = None;
    let mut causes = std::collections::BTreeSet::new();
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
            let verdict = match first_unchecked {
                None => Verdict::Intact { rows: checked },
                Some(first_id) => Verdict::Unchecked {
                    rows: checked,
                    unchecked,
                    first_id,
                    causes: causes.into_iter().collect(),
                },
            };
            return Ok((verdict, head));
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
            match chain.check(&digest, previous.as_deref(), &ts, &event) {
                RowCheck::Valid => {}
                RowCheck::Mismatch => {
                    return Ok((
                        Verdict::Broken {
                            id,
                            reason: format!("digest of row {checked} does not match its content"),
                        },
                        head,
                    ));
                }
                // The link to the previous row was already checked from the stored digests, so
                // the scan goes on: a break further down is still found.
                RowCheck::KeyNotHeld(key) => {
                    unchecked += 1;
                    first_unchecked.get_or_insert(id);
                    causes.insert(format!("key {key} is not held"));
                }
                RowCheck::SchemeUnknown(scheme) => {
                    unchecked += 1;
                    first_unchecked.get_or_insert(id);
                    causes.insert(format!("scheme {scheme} is not understood by this build"));
                }
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

    fn actor() -> Actor {
        Actor {
            roles: vec!["clinician".into()],
            organisations: vec!["riverside".into()],
        }
    }

    #[test]
    fn the_request_actor_is_added_to_the_detail_and_overwrites_a_handlers_claim() {
        let mut e = ev();
        e.detail = Some(serde_json::json!({"n": 1, "actor": {"roles": ["admin"]}}));
        let out = ACTOR.sync_scope(actor(), || with_actor(&e));
        let d = out.detail.unwrap();
        assert_eq!(d["n"], 1);
        assert_eq!(d["actor"]["roles"], serde_json::json!(["clinician"]));
        assert_eq!(
            d["actor"]["organisations"],
            serde_json::json!(["riverside"])
        );
        // No detail: one is made.
        let mut bare = ev();
        bare.detail = None;
        let out = ACTOR.sync_scope(actor(), || with_actor(&bare));
        assert_eq!(out.detail.unwrap()["actor"]["roles"][0], "clinician");
    }

    #[test]
    fn outside_a_request_the_event_is_unchanged() {
        let e = ev();
        assert_eq!(with_actor(&e), e);
    }

    /// A chain whose only key has the id `id`.
    fn keyed_as(id: &str) -> Chain {
        Chain::new(KeySet::load_with_domains(
            &KeyConfig::new("t", "T"),
            Some(KEY),
            None,
            Some(id),
            &[],
        ))
    }

    #[test]
    fn a_row_under_a_key_we_do_not_hold_is_not_a_mismatch() {
        let e = ev();
        let digest = keyed_as("k1").digest(None, "2026-01-01T00:00:00.000000Z", &e);
        assert!(digest.starts_with("d1.k1:"), "{digest}");
        let check = |c: &Chain, d: &str| c.check(d, None, "2026-01-01T00:00:00.000000Z", &e);
        assert_eq!(check(&keyed_as("k1"), &digest), RowCheck::Valid);
        // Another key id: cannot be checked, which is not evidence of tampering.
        assert_eq!(
            check(&keyed_as("k2"), &digest),
            RowCheck::KeyNotHeld("k1".into())
        );
        // No key at all: the same.
        assert_eq!(
            check(&Chain::unkeyed(), &digest),
            RowCheck::KeyNotHeld("k1".into())
        );
        // A scheme this build does not know.
        let future = digest.replacen("d1.", "d9.", 1);
        assert_eq!(
            check(&keyed_as("k1"), &future),
            RowCheck::SchemeUnknown("d9".into())
        );
        assert_eq!(
            check(&Chain::unkeyed(), &future),
            RowCheck::SchemeUnknown("d9".into())
        );
    }

    #[test]
    fn a_wrong_mac_under_a_held_key_and_garbage_are_still_mismatches() {
        let e = ev();
        let at = "2026-01-01T00:00:00.000000Z";
        let c = keyed_as("k1");
        let digest = c.digest(None, at, &e);
        let mut other = e.clone();
        other.agent = "someone-else".into();
        assert_eq!(c.check(&digest, None, at, &other), RowCheck::Mismatch);
        for junk in ["", "nonsense", "d1.k1:", ":abcd", "d1.k1:zz"] {
            assert_eq!(c.check(junk, None, at, &e), RowCheck::Mismatch, "{junk:?}");
        }
        assert_eq!(
            Chain::unkeyed().check("nonsense", None, at, &e),
            RowCheck::Mismatch
        );
        // An unkeyed digest in a keyed chain is a downgrade someone without the key can write.
        let plain = Chain::unkeyed().digest(None, at, &e);
        assert_eq!(c.check(&plain, None, at, &e), RowCheck::Mismatch);
    }
}

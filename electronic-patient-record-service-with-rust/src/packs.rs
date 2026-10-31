//! Country packs, first slice (task OS-10; `spec/interoperability/index.md` "Pack mechanism").
//!
//! A pack is data: `packs/<id>/pack.toml`, `phone.toml` and an optional `identifiers.toml` (the Australian IHI
//! rule, off unless a pack turns it on). Only these two keys are built. The
//! packs a deployment enables (`EPR_PACKS`, comma-separated ids) are loaded once at start and checked: the
//! files must have exactly the known keys, the id must match the directory, the code must be 1 to 999, and
//! the pack's own vectors must pass through the real matcher (a pack with no vectors does not load). Two
//! enabled packs that set different calling codes stop startup unless the deployer names the winner
//! (`EPR_PACK_WINNER_PHONE_CALLING_CODE`, a pack id). With no pack enabled nothing changes: the matcher's
//! own default applies. The effective rules have a SHA-256, logged at start; it is not yet written on the
//! records the rules influenced (that needs a migration).

use electronic_patient_record_matcher::{
    Decision, HumanName, IHI_SYSTEM, Identifier, MatchConfig, MatchingEngine, Patient, Telecom,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::OnceLock;

/// `pack.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    id: String,
    name: String,
    version: String,
    reviewed: String,
}

/// A pair of phone numbers a pack says must (or must not) match.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    local: String,
    international: String,
}

/// `phone.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Phone {
    calling_code: u16,
    #[serde(default, rename = "match")]
    matches: Vec<Pair>,
    #[serde(default)]
    differ: Vec<Pair>,
}

/// Two IHIs a pack says must link (the same number) or must not (different numbers).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IhiPair {
    a: String,
    b: String,
}

/// `identifiers.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identifiers {
    use_ihi: bool,
    #[serde(default, rename = "match")]
    matches: Vec<IhiPair>,
    #[serde(default)]
    differ: Vec<IhiPair>,
}

/// One loaded pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The pack id (its directory name).
    pub id: String,
    /// Its version.
    pub version: String,
    /// Whether a person has reviewed it: `none` until someone does.
    pub reviewed: String,
    /// The calling code it sets.
    pub calling_code: u16,
    /// Whether it turns the IHI rule on.
    pub use_ihi: bool,
}

/// What the enabled packs add up to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleSet {
    /// The packs, in the order enabled.
    pub packs: Vec<Loaded>,
    /// The calling code in force, or `None` for the matcher's default.
    pub calling_code: Option<u16>,
    /// Whether the IHI rule is on: any enabled pack turns it on, none can turn it off.
    pub use_ihi: bool,
}

impl RuleSet {
    /// A SHA-256 over the rules in force, so a log line says which applied.
    #[must_use]
    pub fn hash(&self) -> String {
        let mut text = format!(
            "phone.calling_code={};identifiers.use_ihi={}",
            self.calling_code
                .map_or("default".to_string(), |c| c.to_string()),
            self.use_ihi
        );
        for p in &self.packs {
            text.push_str(&format!(";{}@{}", p.id, p.version));
        }
        Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn phones(a: &str, b: &str) -> (Patient, Patient) {
    let with = |value: &str| Patient {
        telecom: vec![Telecom {
            system: "phone".into(),
            value: value.into(),
        }],
        ..Patient::default()
    };
    (with(a), with(b))
}

/// Whether two phone numbers score as the same under `code`.
fn same_phone(code: u16, a: &str, b: &str) -> bool {
    let engine = MatchingEngine::new(MatchConfig {
        default_calling_code: code,
        ..MatchConfig::default()
    });
    let (x, y) = phones(a, b);
    engine
        .compare(&x, &y)
        .fields
        .iter()
        .any(|f| f.field == "telecom" && f.similarity == Some(1.0))
}

/// Two records alike in every way but their IHIs, compared with the rule on.
fn compare_ihis(a: &str, b: &str) -> Decision {
    let with = |ihi: &str| Patient {
        names: vec![HumanName {
            family: Some("Nguyen".into()),
            given: vec!["Lan".into()],
        }],
        birth_date: Some("1980-03-14".into()),
        identifiers: vec![Identifier {
            system: IHI_SYSTEM.into(),
            value: ihi.into(),
        }],
        ..Patient::default()
    };
    MatchingEngine::new(MatchConfig {
        use_ihi: true,
        ..MatchConfig::default()
    })
    .compare(&with(a), &with(b))
    .decision
}

/// Check `identifiers.toml`, when the pack has one, and say whether it turns the IHI rule on.
fn load_identifiers(root: &Path, id: &str) -> Result<bool, String> {
    let path = root.join("identifiers.toml");
    if !path.exists() {
        return Ok(false);
    }
    let ids: Identifiers = read(&path)?;
    if ids.matches.is_empty() || ids.differ.is_empty() {
        return Err(format!(
            "pack {id}: identifiers.toml needs at least one [[match]] and one [[differ]] vector"
        ));
    }
    for v in ids.matches.iter().chain(&ids.differ) {
        for n in [&v.a, &v.b] {
            if !electronic_patient_record_matcher::is_valid_ihi(n) {
                return Err(format!("pack {id}: {n:?} is not a valid IHI"));
            }
        }
    }
    for v in &ids.matches {
        if v.a != v.b || compare_ihis(&v.a, &v.b) != Decision::AutoLink {
            return Err(format!(
                "pack {id}: IHI vector {:?} and {:?} must link and do not",
                v.a, v.b
            ));
        }
    }
    for v in &ids.differ {
        if v.a == v.b || compare_ihis(&v.a, &v.b) != Decision::NonMatch {
            return Err(format!(
                "pack {id}: IHI vector {:?} and {:?} must not link and do",
                v.a, v.b
            ));
        }
    }
    Ok(ids.use_ihi)
}

fn load_one(dir: &Path, id: &str) -> Result<Loaded, String> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!(
            "pack id {id:?} must be lowercase letters, digits and hyphens"
        ));
    }
    let root = dir.join(id);
    let manifest: Manifest = read(&root.join("pack.toml"))?;
    if manifest.id != id {
        return Err(format!("pack {id}: pack.toml says id {:?}", manifest.id));
    }
    if manifest.name.trim().is_empty() || manifest.version.trim().is_empty() {
        return Err(format!("pack {id}: name and version are required"));
    }
    if !matches!(manifest.reviewed.as_str(), "none" | "reviewed") {
        return Err(format!(
            "pack {id}: reviewed must be \"none\" or \"reviewed\""
        ));
    }
    let phone: Phone = read(&root.join("phone.toml"))?;
    if !(1..=999).contains(&phone.calling_code) {
        return Err(format!("pack {id}: calling_code is outside 1 to 999"));
    }
    if phone.matches.is_empty() || phone.differ.is_empty() {
        return Err(format!(
            "pack {id}: phone.toml needs at least one [[match]] and one [[differ]] vector"
        ));
    }
    for v in &phone.matches {
        if !same_phone(phone.calling_code, &v.local, &v.international) {
            return Err(format!(
                "pack {id}: vector {:?} and {:?} must match and do not",
                v.local, v.international
            ));
        }
    }
    for v in &phone.differ {
        if same_phone(phone.calling_code, &v.local, &v.international) {
            return Err(format!(
                "pack {id}: vector {:?} and {:?} must differ and match",
                v.local, v.international
            ));
        }
    }
    let use_ihi = load_identifiers(&root, id)?;
    Ok(Loaded {
        id: manifest.id,
        version: manifest.version,
        reviewed: manifest.reviewed,
        calling_code: phone.calling_code,
        use_ihi,
    })
}

/// Load and check the enabled packs.
///
/// # Errors
///
/// The first thing wrong: a missing or malformed file, an unknown key, a failed vector, or a calling
/// code two packs disagree on with no winner named.
pub fn load(dir: &Path, enabled: &[String], winner: Option<&str>) -> Result<RuleSet, String> {
    let mut packs = Vec::new();
    for id in enabled {
        if packs.iter().any(|p: &Loaded| &p.id == id) {
            return Err(format!("pack {id} is enabled twice"));
        }
        packs.push(load_one(dir, id)?);
    }
    let mut codes: Vec<u16> = packs.iter().map(|p| p.calling_code).collect();
    codes.sort_unstable();
    codes.dedup();
    let calling_code = match (codes.as_slice(), winner) {
        ([], _) => None,
        ([one], _) => Some(*one),
        (_, Some(w)) => Some(
            packs
                .iter()
                .find(|p| p.id == w)
                .ok_or_else(|| format!("the winner {w:?} is not an enabled pack"))?
                .calling_code,
        ),
        (_, None) => {
            return Err(format!(
                "enabled packs disagree on phone.calling_code ({}); name the winner with EPR_PACK_WINNER_PHONE_CALLING_CODE",
                packs
                    .iter()
                    .map(|p| format!("{}={}", p.id, p.calling_code))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    };
    let use_ihi = packs.iter().any(|p| p.use_ihi);
    Ok(RuleSet {
        packs,
        calling_code,
        use_ihi,
    })
}

/// The enabled pack ids from a setting: comma-separated, blanks ignored.
#[must_use]
pub fn parse_enabled(raw: Option<&str>) -> Vec<String> {
    raw.map(|r| {
        r.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

static RULES: OnceLock<RuleSet> = OnceLock::new();

/// Load the packs the environment enables (`EPR_PACKS`, `EPR_PACKS_DIR` default `packs`) and keep the
/// result for the process. Called once at start.
///
/// # Errors
///
/// The reason a pack did not load; the service must not start.
pub fn init() -> Result<&'static RuleSet, String> {
    let enabled = parse_enabled(std::env::var("EPR_PACKS").ok().as_deref());
    let dir = std::env::var("EPR_PACKS_DIR").unwrap_or_else(|_| "packs".to_string());
    let winner = std::env::var("EPR_PACK_WINNER_PHONE_CALLING_CODE").ok();
    let rules = load(Path::new(&dir), &enabled, winner.as_deref())?;
    for p in &rules.packs {
        tracing::warn!(pack = %p.id, version = %p.version, reviewed = %p.reviewed, "pack enabled; reviewed none means no person who works with that country's standards has signed it");
    }
    tracing::info!(rules = %rules.hash(), packs = rules.packs.len(), "effective pack rules");
    Ok(RULES.get_or_init(|| rules))
}

/// The matching engine for the rules in force: the matcher's default when no pack is enabled.
#[must_use]
pub fn engine() -> MatchingEngine {
    let rules = RULES.get();
    MatchingEngine::new(MatchConfig {
        default_calling_code: rules
            .and_then(|r| r.calling_code)
            .unwrap_or(MatchConfig::default().default_calling_code),
        use_ihi: rules.is_some_and(|r| r.use_ihi),
        ..MatchConfig::default()
    })
}

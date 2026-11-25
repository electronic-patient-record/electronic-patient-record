//! Integration tests, folded into one test binary.

#[path = "main/codes.rs"]
mod codes;
#[path = "main/config.rs"]
mod config;
#[path = "main/demo_binary.rs"]
mod demo_binary;
#[path = "main/fields.rs"]
mod fields;
#[path = "main/household.rs"]
mod household;
#[path = "main/identifier.rs"]
mod identifier;
#[path = "main/ihi.rs"]
mod ihi;
#[path = "main/marks.rs"]
mod marks;
#[path = "main/matching.rs"]
mod matching;
#[path = "main/names.rs"]
mod names;
#[path = "main/nicknames.rs"]
mod nicknames;
#[path = "main/phone.rs"]
mod phone;
#[path = "main/phonetic.rs"]
mod phonetic;
#[path = "main/placeholders.rs"]
mod placeholders;
#[path = "main/properties.rs"]
mod properties;
#[path = "main/swap.rs"]
mod swap;
#[path = "main/temporary_identity.rs"]
mod temporary_identity;
#[path = "main/transliteration.rs"]
mod transliteration;
#[path = "main/welsh.rs"]
mod welsh;

/// Whether the one thing that keeps a pair from being linked automatically is that they share no verified identifier
/// (0.9.0): every other rule is met. Before 0.9.0 such a pair was `AutoLink`.
fn only_an_identifier_is_missing(r: &electronic_patient_record_matcher::MatchResult) -> bool {
    use electronic_patient_record_matcher::{ConflictCode, Decision};
    r.decision == Decision::Review && r.conflict_codes == [ConflictCode::NoSharedIdentifier]
}

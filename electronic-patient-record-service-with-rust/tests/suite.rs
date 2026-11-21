//! One test program for the integration tests that share nothing process-wide (LC-15). Each file in `tests/` used to
//! be its own program, built and linked against the library after every edit (about 44 s each, measured 2026-11-17).
//! These files stay where they are, so every path to them still holds; each is a module here. A file that sets the
//! process's environment (the auth settings), holds process-wide state, or that a script or a document runs by name
//! (`cargo test --test <name>`) stays its own program, listed in `Cargo.toml`. `test_targets` checks that every file
//! in `tests/` is one or the other.
//!
//! Run one file's tests with `cargo test --test suite <module>::`.

#[path = "advisory_expiry.rs"]
mod advisory_expiry;
#[path = "allergies_store.rs"]
mod allergies_store;
#[path = "amendments_store.rs"]
mod amendments_store;
#[path = "audit_chain.rs"]
mod audit_chain;
#[path = "audit_partitions.rs"]
mod audit_partitions;
#[path = "authz_audit.rs"]
mod authz_audit;
#[path = "axa_defaults.rs"]
mod axa_defaults;
#[path = "break_glass_store.rs"]
mod break_glass_store;
#[path = "consents_store.rs"]
mod consents_store;
#[path = "current_views.rs"]
mod current_views;
#[path = "db_commands.rs"]
mod db_commands;
#[path = "death_scenarios.rs"]
mod death_scenarios;
#[path = "death_store.rs"]
mod death_store;
#[path = "destroy_refused.rs"]
mod destroy_refused;
#[path = "doc_census.rs"]
mod doc_census;
#[path = "drill_records.rs"]
mod drill_records;
#[path = "error_census.rs"]
mod error_census;
#[path = "fhir_census.rs"]
mod fhir_census;
#[path = "grief_census.rs"]
mod grief_census;
#[path = "hazards_store.rs"]
mod hazards_store;
#[path = "keycloak_guard.rs"]
mod keycloak_guard;
#[path = "label_census.rs"]
mod label_census;
#[path = "links_store.rs"]
mod links_store;
#[path = "migration_down.rs"]
mod migration_down;
#[path = "observations_store.rs"]
mod observations_store;
#[path = "outbound_census.rs"]
mod outbound_census;
#[path = "packs.rs"]
mod packs;
#[path = "pas_contract.rs"]
mod pas_contract;
#[path = "patients_store.rs"]
mod patients_store;
#[path = "personal_data_inventory.rs"]
mod personal_data_inventory;
#[path = "photos_store.rs"]
mod photos_store;
#[path = "readiness.rs"]
mod readiness;
#[path = "referral_unmatched.rs"]
mod referral_unmatched;
#[path = "referral_worklist.rs"]
mod referral_worklist;
#[path = "referrals_store.rs"]
mod referrals_store;
#[path = "relationships_store.rs"]
mod relationships_store;
#[path = "risk_field_census.rs"]
mod risk_field_census;
#[path = "route_census.rs"]
mod route_census;
#[path = "sql_census.rs"]
mod sql_census;
#[path = "stored_words.rs"]
mod stored_words;
#[path = "support_only.rs"]
mod support_only;
#[path = "tasks_store.rs"]
mod tasks_store;
#[path = "test_targets.rs"]
mod test_targets;

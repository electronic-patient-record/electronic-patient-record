//! One test program for the integration tests that share nothing process-wide (LC-15). Each file in `tests/` used to
//! be its own program, built and linked against the library after every edit (about 44 s each, measured 2026-11-17).
//! These files stay where they are, so every path to them still holds; each is a module here. A file that sets the
//! process's environment (the auth settings), holds process-wide state, or that a script or a document runs by name
//! (`cargo test --test <name>`) stays its own program, listed in `Cargo.toml`. `test_targets` checks that every file
//! in `tests/` is one or the other.
//!
//! Run one file's tests with `cargo test --test main <module>::`.

// Shared by more than one file below, so declared once here.
#[path = "audit_switch/mod.rs"]
mod audit_switch;
#[path = "chaos_support/mod.rs"]
mod chaos_support;
#[path = "http_log_privacy.rs"]
mod http_log_privacy;
#[path = "openapi_breaks.rs"]
mod openapi_breaks;
#[path = "openapi_support/mod.rs"]
mod openapi_support;

#[path = "advisory_expiry.rs"]
mod advisory_expiry;
#[path = "allergies_store.rs"]
mod allergies_store;
#[path = "amendments_store.rs"]
mod amendments_store;
#[path = "audit_attribution.rs"]
mod audit_attribution;
#[path = "audit_chain.rs"]
mod audit_chain;
#[path = "audit_coverage.rs"]
mod audit_coverage;
#[path = "audit_immutable.rs"]
mod audit_immutable;
#[path = "audit_partitions.rs"]
mod audit_partitions;
#[path = "audit_search_coverage.rs"]
mod audit_search_coverage;
#[path = "audit_words.rs"]
mod audit_words;
#[path = "authz_audit.rs"]
mod authz_audit;
#[path = "axa_defaults.rs"]
mod axa_defaults;
#[path = "balp_claim.rs"]
mod balp_claim;
#[path = "break_glass_store.rs"]
mod break_glass_store;
#[path = "candidate_recall.rs"]
mod candidate_recall;
#[path = "chaos_api.rs"]
mod chaos_api;
#[path = "chaos_roles.rs"]
mod chaos_roles;
#[path = "consents_store.rs"]
mod consents_store;
#[path = "current_views.rs"]
mod current_views;
#[path = "database_down_sweep.rs"]
mod database_down_sweep;
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
#[path = "entries_store.rs"]
mod entries_store;
#[path = "error_census.rs"]
mod error_census;
#[path = "fhir_audit_paging.rs"]
mod fhir_audit_paging;
#[path = "fhir_census.rs"]
mod fhir_census;
#[path = "grief_census.rs"]
mod grief_census;
#[path = "hazards_store.rs"]
mod hazards_store;
#[path = "hl7_samples.rs"]
mod hl7_samples;
#[path = "http_adjustments.rs"]
mod http_adjustments;
#[path = "http_amendments.rs"]
mod http_amendments;
#[path = "http_api_tokens.rs"]
mod http_api_tokens;
#[path = "http_assessments.rs"]
mod http_assessments;
#[path = "http_audit_refusal.rs"]
mod http_audit_refusal;
#[path = "http_audit_refusal_roles.rs"]
mod http_audit_refusal_roles;
#[path = "http_conflicts.rs"]
mod http_conflicts;
#[path = "http_death_reports.rs"]
mod http_death_reports;
#[path = "http_deaths.rs"]
mod http_deaths;
#[path = "http_entries.rs"]
mod http_entries;
#[path = "http_feed_health.rs"]
mod http_feed_health;
#[path = "http_hazards.rs"]
mod http_hazards;
#[path = "http_idempotency.rs"]
mod http_idempotency;
#[path = "http_idempotency_roles.rs"]
mod http_idempotency_roles;
#[path = "http_lockout.rs"]
mod http_lockout;
#[path = "http_observations.rs"]
mod http_observations;
#[path = "http_overdue_reviews.rs"]
mod http_overdue_reviews;
#[path = "http_patients.rs"]
mod http_patients;
#[path = "http_permission_basis.rs"]
mod http_permission_basis;
#[path = "http_proxy_scopes.rs"]
mod http_proxy_scopes;
#[path = "http_rate_limit.rs"]
mod http_rate_limit;
#[path = "http_referral_queue.rs"]
mod http_referral_queue;
#[path = "http_referral_worklist.rs"]
mod http_referral_worklist;
#[path = "http_referrals.rs"]
mod http_referrals;
#[path = "http_results.rs"]
mod http_results;
#[path = "http_role_views.rs"]
mod http_role_views;
#[path = "http_roles.rs"]
mod http_roles;
#[path = "http_suspicions.rs"]
mod http_suspicions;
#[path = "http_tasks.rs"]
mod http_tasks;
#[path = "http_typing.rs"]
mod http_typing;
#[path = "http_watch_list.rs"]
mod http_watch_list;
#[path = "identifier_keys.rs"]
mod identifier_keys;
#[path = "keycloak_guard.rs"]
mod keycloak_guard;
#[path = "label_census.rs"]
mod label_census;
#[path = "licence_files.rs"]
mod licence_files;
#[path = "links_store.rs"]
mod links_store;
#[path = "metrics.rs"]
mod metrics;
#[path = "migration_down.rs"]
mod migration_down;
#[path = "migration_relative_history.rs"]
mod migration_relative_history;
#[path = "mllp_feed.rs"]
mod mllp_feed;
#[path = "observations_store.rs"]
mod observations_store;
#[path = "openapi_census.rs"]
mod openapi_census;
#[path = "openapi_responses.rs"]
mod openapi_responses;
#[path = "openapi_responses_roles.rs"]
mod openapi_responses_roles;
#[path = "outbound_census.rs"]
mod outbound_census;
#[path = "outbox_store.rs"]
mod outbox_store;
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
#[path = "purpose_sweep.rs"]
mod purpose_sweep;
#[path = "races.rs"]
mod races;
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
#[path = "relative_history_census.rs"]
mod relative_history_census;
#[path = "risk_field_census.rs"]
mod risk_field_census;
#[path = "route_census.rs"]
mod route_census;
#[path = "runtime_boot.rs"]
mod runtime_boot;
#[path = "shared_place.rs"]
mod shared_place;
#[path = "sql_census.rs"]
mod sql_census;
#[path = "start_binary.rs"]
mod start_binary;
#[path = "stored_words.rs"]
mod stored_words;
#[path = "support_only.rs"]
mod support_only;
#[path = "tasks_store.rs"]
mod tasks_store;
#[path = "test_targets.rs"]
mod test_targets;

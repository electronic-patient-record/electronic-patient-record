//! Electronic patient record service (Axum, `axa`, PostgreSQL).
//!
//! Scaffold (milestone M0): the app wiring, a secure-by-default PASETO guard,
//! the append-only audit table migration and one endpoint over the matcher.
//! The clinical record and the FHIR facade are planned in `tasks.md` and are not built yet.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// The database migrations, embedded in the binary: `migrations/*.sql`. They have no down files, so the
/// clinical record cannot be rolled back through them.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub use electronic_patient_record_domain::adjustments;
pub mod adjustments_store;
pub mod adt_feed;
pub mod allergies;
pub use electronic_patient_record_domain::amendment;
pub mod amendments_store;
pub mod api_error;
pub mod api_tokens_store;
pub mod app;
pub use electronic_patient_record_domain::assessments;
pub mod assessments_store;
pub mod audit;
pub mod audit_retention;
pub mod auth;
pub mod authz;
pub mod break_glass;
pub use electronic_patient_record_domain::clock;
pub use electronic_patient_record_domain::coding;
pub use electronic_patient_record_domain::consent;
pub mod consents_store;
pub mod cursor;
pub use electronic_patient_record_domain::treatment_refusal;
pub mod controllers;
pub mod treatment_refusals_store;
pub use electronic_patient_record_domain::dates;
pub use electronic_patient_record_domain::death;
pub use electronic_patient_record_domain::decision_ability;
pub mod decision_ability_store;
pub use electronic_patient_record_domain::emergency;
pub mod death_store;
pub mod dependencies;
pub use electronic_patient_record_domain::disclosure_rules;
pub mod disclosure_pack;
pub mod entries_store;
pub mod entry_labels;
pub mod feed_health;
pub mod feed_queue;
pub use electronic_patient_record_domain::gate;
pub use electronic_patient_record_domain::hazard;
pub mod hazard_store;
pub mod idempotency;
pub use electronic_patient_record_domain::ids;
pub mod interop;
pub mod json;
pub use electronic_patient_record_domain::labelling;
pub use electronic_patient_record_domain::link_token;
pub mod links;
pub mod list_limits;
pub mod lockout;
pub mod metrics;
pub use electronic_patient_record_domain::masking;
pub use electronic_patient_record_domain::missed;
pub mod missed_reasons;
pub mod mllp;
pub use electronic_patient_record_domain::observations;
pub mod observations_store;
pub mod openapi;
pub mod outbox;
pub use electronic_patient_record_domain::packs;
pub use electronic_patient_record_domain::page;
pub use electronic_patient_record_domain::pas_suspicion;
pub use electronic_patient_record_domain::pathways;
pub mod patients_store;
pub mod permission;
pub mod pg_time;
pub mod photos_store;
pub use electronic_patient_record_domain::policy;
pub mod pool;
pub use electronic_patient_record_domain::privacy;
pub use electronic_patient_record_domain::problem;
pub use electronic_patient_record_domain::proxy;
pub mod rate_limit;
pub use electronic_patient_record_domain::referral;
pub mod referral_timeouts;
pub mod referral_unmatched;
pub mod referral_worklist;
pub mod referrals_store;
pub mod relationships;
pub use electronic_patient_record_domain::relative_history;
pub mod runtime;
pub use electronic_patient_record_domain::results;
pub mod results_store;
pub use electronic_patient_record_domain::role_views;
pub mod routes;
pub mod schema_match;
pub mod site;
pub mod store;
pub use electronic_patient_record_domain::suspicion;
pub use electronic_patient_record_domain::task_rules;
pub mod suspicions_store;
pub mod task_reminders;
pub mod tasks_store;
pub mod terminology;
pub use electronic_patient_record_domain::typing;
pub use electronic_patient_record_domain::watch_list;
pub mod watch_list_store;

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

pub mod adjustments;
pub mod adjustments_store;
pub mod adt_feed;
pub mod allergies;
pub mod amendment;
pub mod amendments_store;
pub mod api_error;
pub mod app;
pub mod audit;
pub mod audit_retention;
pub mod auth;
pub mod authz;
pub mod break_glass;
pub mod clock;
pub mod consent;
pub mod consents_store;
pub mod controllers;
pub mod death;
pub mod death_store;
pub mod dependencies;
pub mod feed_queue;
pub mod gate;
pub mod hazard;
pub mod hazard_store;
pub mod idempotency;
pub mod ids;
pub mod interop;
pub mod link_token;
pub mod links;
pub mod lockout;
pub mod metrics;
pub mod mllp;
pub mod observations;
pub mod observations_store;
pub mod openapi;
pub mod outbox;
pub mod packs;
pub mod page;
pub mod patients_store;
pub mod permission;
pub mod photos_store;
pub mod policy;
pub mod pool;
pub mod privacy;
pub mod proxy;
pub mod rate_limit;
pub mod referral;
pub mod referral_timeouts;
pub mod referral_unmatched;
pub mod referral_worklist;
pub mod referrals_store;
pub mod relationships;
pub mod results;
pub mod results_store;
pub mod routes;
pub mod site;
pub mod store;
pub mod terminology;

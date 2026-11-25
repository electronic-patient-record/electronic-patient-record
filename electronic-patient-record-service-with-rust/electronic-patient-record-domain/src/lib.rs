//! Electronic patient record domain rules: what the record allows, refuses and works out, with no database and no
//! web. Policy and consent decide who may see what; the referral, pathway and suspicion rules say what happens
//! next; observations, results, problems, relative history, adjustments, amendments, deaths and assessments say what a valid
//! entry is; the
//! hazard register's rules say who may retire a hazard. Each rule is a plain function of its inputs, so it is tested
//! without Postgres and built once for every edit to a route or a store.
//!
//! The service (`electronic-patient-record`) stores, serves and audits; it re-exports each module here under its
//! old path, so `electronic_patient_record::consent` and this crate's `consent` are one module.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod adjustments;
pub mod amendment;
pub mod assessments;
pub mod clock;
pub mod coding;
pub mod consent;
pub mod dates;
pub mod death;
pub mod gate;
pub mod hazard;
pub mod ids;
pub mod link_token;
pub mod observations;
pub mod packs;
pub mod page;
pub mod pas_suspicion;
pub mod pathways;
pub mod permission;
pub mod policy;
pub mod privacy;
pub mod problem;
pub mod proxy;
pub mod referral;
pub mod relative_history;
pub mod results;
pub mod role_views;
pub mod site;
pub mod suspicion;
pub mod task_rules;
pub mod typing;
pub mod watch_list;

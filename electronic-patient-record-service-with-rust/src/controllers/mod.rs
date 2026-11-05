//! HTTP controllers.

pub mod adjustments;
pub mod allergies;
pub mod amendments;
pub mod audit;
pub mod break_glass;
mod common;
pub mod consents;
pub mod deaths;
pub mod feed;
pub mod fhir;
pub mod fhir_r5;
pub mod hazards;
pub mod links;
pub mod me;
pub mod metrics;
pub mod observations;
pub mod patients;
pub mod photos;
pub mod referral_queue;
pub mod referral_worklist;
pub mod referrals;
pub mod relationships;

use axa::routes::AppRoutes;

/// Every group of routes the service serves, the health routes first. `App::routes` and the route census
/// both read this, so the census sees exactly what is served.
#[must_use]
pub fn route_groups() -> Vec<axa::routes::Routes> {
    vec![
        axa::health::routes(),
        patients::routes(),
        audit::routes(),
        me::routes(),
        metrics::routes(),
        crate::openapi::routes(),
        fhir::routes(),
        fhir_r5::routes(),
        feed::routes(),
        break_glass::routes(),
        hazards::routes(),
        referral_queue::routes(),
        referral_worklist::routes(),
    ]
}

/// Every route the service serves, as one table for the router.
#[must_use]
pub fn all_routes() -> AppRoutes {
    route_groups()
        .into_iter()
        .fold(AppRoutes::empty(), AppRoutes::add_route)
}

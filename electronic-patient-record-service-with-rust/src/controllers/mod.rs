//! HTTP controllers.

pub mod allergies;
pub mod amendments;
pub mod audit;
pub mod break_glass;
mod common;
pub mod consents;
pub mod feed;
pub mod fhir;
pub mod hazards;
pub mod links;
pub mod me;
pub mod metrics;
pub mod observations;
pub mod patients;
pub mod photos;
pub mod relationships;

use loco_rs::controller::AppRoutes;

/// Every route the service serves. `App::routes` and the route census test both read this, so
/// the census sees exactly what is served.
#[must_use]
pub fn all_routes() -> AppRoutes {
    AppRoutes::with_default_routes()
        .add_route(patients::routes())
        .add_route(audit::routes())
        .add_route(me::routes())
        .add_route(metrics::routes())
        .add_route(fhir::routes())
        .add_route(feed::routes())
        .add_route(break_glass::routes())
        .add_route(hazards::routes())
}

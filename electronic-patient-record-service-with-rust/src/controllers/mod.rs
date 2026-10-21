//! HTTP controllers.

pub mod allergies;
pub mod audit;
pub mod consents;
pub mod fhir;
pub mod me;
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
        .add_route(fhir::routes())
}

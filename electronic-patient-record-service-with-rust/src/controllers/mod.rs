//! HTTP controllers.

pub mod audit;
pub mod consents;
pub mod me;
pub mod patients;
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
}

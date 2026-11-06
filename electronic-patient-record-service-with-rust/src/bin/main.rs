//! `electronic-patient-record` binary entrypoint.

#![forbid(unsafe_code)]

use electronic_patient_record::app::App;

/// Run the command line for this application: `start`, `db migrate` and the rest.
#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(axa::cli::parse(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
    ))
    .await
}

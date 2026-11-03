//! `electronic-patient-record` binary entrypoint: boots the loco.rs CLI.

#![forbid(unsafe_code)]

use electronic_patient_record::app::App;
use electronic_patient_record_migration::Migrator;
use loco_rs::cli;

/// Run the loco command line for this application, with its migrations: `start`, `db migrate` and the rest.
#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}

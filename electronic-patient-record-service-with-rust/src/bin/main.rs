//! `electronic-patient-record` binary entrypoint: boots the loco.rs CLI.

#![forbid(unsafe_code)]

use electronic_patient_record::app::App;
use electronic_patient_record_migration::Migrator;
use loco_rs::cli;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}

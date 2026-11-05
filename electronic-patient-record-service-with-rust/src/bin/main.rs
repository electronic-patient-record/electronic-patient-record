//! `electronic-patient-record` binary entrypoint.

#![forbid(unsafe_code)]

use axa::cli::Cli;
use clap::Parser;
use electronic_patient_record::app::App;

/// Run the command line for this application: `start`, `db migrate` and the rest.
#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(Cli::parse()).await
}

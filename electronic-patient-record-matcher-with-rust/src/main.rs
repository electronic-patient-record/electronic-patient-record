//! Demo: compare two patients given as JSON files, print the explainable result.

use electronic_patient_record_matcher::{MatchingEngine, Patient};

fn load(path: &str) -> Result<Patient, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [a, b] = args.as_slice() else {
        eprintln!("usage: electronic-patient-record-matcher <patient-a.json> <patient-b.json>");
        std::process::exit(2);
    };
    let result = MatchingEngine::default().compare(&load(a)?, &load(b)?);
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

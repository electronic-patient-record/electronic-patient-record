//! `db reset`, `db down` and `db truncate` are refused by the binary itself, before it reads a setting
//! or opens a connection. Database-free: this runs the real program, because the refusal is in `main`, on
//! the way to the code a unit test can reach.

use std::process::{Command, Output};

const BINARY: &str = env!("CARGO_BIN_EXE_electronic-patient-record");

/// Run `db <command>` with only the named variables set, and no database to reach.
fn db(command: &str, vars: &[(&str, &str)]) -> Output {
    Command::new(BINARY)
        .args(["db", command])
        .env_clear()
        .envs(vars.iter().copied())
        .output()
        .expect("the binary runs")
}

fn refused_by_axa(output: &Output) -> bool {
    !output.status.success()
        && String::from_utf8_lossy(&output.stderr).contains("AXA_DESTROY_DATA=true")
}

#[test]
fn nothing_that_destroys_data_runs_in_production_even_when_confirmed() {
    let vars = [("APP_ENV", "production"), ("AXA_DESTROY_DATA", "true")];
    for command in ["reset", "down", "truncate"] {
        assert!(refused_by_axa(&db(command, &vars)), "db {command}");
    }
}

#[test]
fn development_needs_the_confirmation() {
    let vars = [("APP_ENV", "development")];
    for command in ["reset", "down", "truncate"] {
        assert!(refused_by_axa(&db(command, &vars)), "db {command}");
    }
}

#[test]
fn the_old_flag_confirms_nothing() {
    let vars = [
        ("APP_ENV", "development"),
        ("EPR_ALLOW_DESTRUCTIVE_DOWN", "true"),
    ];
    assert!(refused_by_axa(&db("reset", &vars)));
}

#[test]
fn an_option_cannot_talk_a_production_process_round_it() {
    let output = Command::new(BINARY)
        .args(["--environment", "test", "db", "reset"])
        .env_clear()
        .env("APP_ENV", "production")
        .env("AXA_DESTROY_DATA", "true")
        .output()
        .expect("the binary runs");
    assert!(refused_by_axa(&output));
}

//! The HL7 v2 sample messages (`operations/hl7/samples`, IH-11): every one reads, and the reader's acknowledgement for
//! each is exactly the committed golden file in `tests/golden/hl7-acks/`. No database: these are the reader's answers,
//! before matching (the feed's own tests cover what the database decides). A golden file is rewritten only with
//! `EPR_BLESS=1`, when a change is meant, and reviewed in the diff; never to make a failing test pass.

use electronic_patient_record::interop::hl7_adt::{self, Rejection};
use std::fs;
use std::path::{Path, PathBuf};

fn samples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../operations/hl7/samples")
}

fn golden() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/hl7-acks")
}

/// A sample as the sender sends it: line breaks as `\r`, the placeholders filled with fixed values.
fn message(text: &str) -> String {
    text.trim_end()
        .lines()
        .collect::<Vec<_>>()
        .join("\r")
        .replace("{{control}}", "SAMPLE-1")
        .replace("{{mrn}}", "M-0001")
}

fn names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(samples())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|n| n.strip_suffix(".hl7").map(str::to_string))
        .collect();
    names.sort();
    names
}

#[test]
fn every_sample_reads_and_is_answered_as_its_golden_file_says() {
    let bless = std::env::var("EPR_BLESS").is_ok_and(|v| v == "1");
    let names = names();
    assert!(names.len() >= 10, "the sample set is there: {names:?}");
    fs::create_dir_all(golden()).unwrap();
    let mut differ = Vec::new();
    for name in &names {
        let text = message(&fs::read_to_string(samples().join(format!("{name}.hl7"))).unwrap());
        let read = hl7_adt::patient_from_adt(&text);
        assert!(
            !matches!(read, Err(Rejection::NotHl7(_))),
            "{name} does not read as HL7"
        );
        let ack = hl7_adt::acknowledgement(&text, &read, "ACK-1", "20261117000000")
            .unwrap_or_else(|| panic!("{name} has no acknowledgement"));
        let file = golden().join(format!("{name}.ack"));
        // One segment a line in the file, so a diff is readable; the comparison is of the exact text.
        let shown = ack.replace('\r', "\n") + "\n";
        if bless {
            fs::write(&file, &shown).unwrap();
        } else {
            match fs::read_to_string(&file) {
                Ok(expected) if expected == shown => {}
                Ok(expected) => differ.push(format!("{name}:\n  expected {expected:?}\n  got      {shown:?}")),
                Err(_) => differ.push(format!("{name}: no golden file; run EPR_BLESS=1 cargo test --test hl7_samples and review it")),
            }
        }
    }
    assert!(
        differ.is_empty(),
        "acknowledgements changed:\n{}",
        differ.join("\n")
    );
    // A golden file with no sample is stale.
    for entry in fs::read_dir(golden()).unwrap() {
        let n = entry.unwrap().file_name().into_string().unwrap();
        let stem = n.strip_suffix(".ack").unwrap_or(&n).to_string();
        assert!(names.contains(&stem), "golden file {n} has no sample");
    }
}

#[test]
fn the_samples_say_what_their_names_say() {
    let code = |name: &str| {
        let text = message(&fs::read_to_string(samples().join(format!("{name}.hl7"))).unwrap());
        let read = hl7_adt::patient_from_adt(&text);
        let ack = hl7_adt::acknowledgement(&text, &read, "ACK-1", "20261117000000").unwrap();
        let msa = ack
            .split('\r')
            .find(|l| l.starts_with("MSA|"))
            .unwrap()
            .to_string();
        (msa.split('|').nth(1).unwrap().to_string(), read)
    };
    for ok in [
        "a28-register",
        "a08-update",
        "a01-admit",
        "a04-outpatient",
        "a31-update-person",
        "a08-death",
        "a28-training",
    ] {
        assert_eq!(code(ok).0, "AA", "{ok}");
    }
    assert!(
        code("a08-death").1.unwrap().death.is_some(),
        "the death sample reports a death"
    );
    assert_eq!(code("oru-not-handled").0, "AR");
    assert_eq!(code("no-pid").0, "AE");
    assert_eq!(code("bad-national-number").0, "AE");
    // Synthetic only: no sample carries anything but the test number and test names.
    for name in names() {
        let text = fs::read_to_string(samples().join(format!("{name}.hl7"))).unwrap();
        assert!(
            text.contains("Sample^") || name == "no-pid",
            "{name} uses the test family name"
        );
    }
}

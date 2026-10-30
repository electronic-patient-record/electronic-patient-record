//! Country packs (OS-10, first slice): the packs in `packs/` load and their vectors pass; a pack with a
//! wrong code, no vectors, an unknown key, or a mismatched id does not load; two packs that disagree
//! stop startup unless a winner is named; with none enabled the matcher's own default stands, and a
//! pack that sets another code changes what a local phone number means. Database-free.

use electronic_patient_record::packs::{self, parse_enabled};
use electronic_patient_record_matcher::{MatchingEngine, Patient, Telecom};
use std::path::{Path, PathBuf};

fn shipped() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs")
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

/// A scratch packs directory holding one pack with the given files.
fn scratch(id: &str, manifest: &str, phone: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "epr-packs-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join(id)).unwrap();
    std::fs::write(dir.join(id).join("pack.toml"), manifest).unwrap();
    std::fs::write(dir.join(id).join("phone.toml"), phone).unwrap();
    dir
}

const MANIFEST: &str = "id = \"t\"\nname = \"T\"\nversion = \"0.1.0\"\nreviewed = \"none\"\n";
const PHONE: &str = "calling_code = 44\n[[match]]\nlocal = \"07700 900123\"\ninternational = \"+44 7700 900123\"\n[[differ]]\nlocal = \"07700 900123\"\ninternational = \"+61 7700 900123\"\n";

fn phone_patient(v: &str) -> Patient {
    Patient {
        telecom: vec![Telecom {
            system: "phone".into(),
            value: v.into(),
        }],
        ..Patient::default()
    }
}

#[test]
fn the_shipped_packs_load_and_say_they_are_unreviewed() {
    let rules = packs::load(&shipped(), &ids(&["uk"]), None).unwrap();
    assert_eq!(rules.calling_code, Some(44));
    assert_eq!(rules.packs[0].reviewed, "none");
    let au = packs::load(&shipped(), &ids(&["au"]), None).unwrap();
    assert_eq!(au.calling_code, Some(61));
}

#[test]
fn with_no_pack_enabled_nothing_changes() {
    let rules = packs::load(&shipped(), &[], None).unwrap();
    assert_eq!(rules.calling_code, None);
    assert!(rules.packs.is_empty());
}

#[test]
fn two_packs_that_disagree_stop_startup_unless_a_winner_is_named() {
    let both = ids(&["uk", "au"]);
    let e = packs::load(&shipped(), &both, None).unwrap_err();
    assert!(e.contains("disagree"), "{e}");
    assert!(e.contains("uk=44") && e.contains("au=61"), "{e}");
    let won = packs::load(&shipped(), &both, Some("au")).unwrap();
    assert_eq!(won.calling_code, Some(61));
    let e = packs::load(&shipped(), &both, Some("fr")).unwrap_err();
    assert!(e.contains("not an enabled pack"), "{e}");
}

#[test]
fn the_rules_have_a_hash_that_changes_with_them() {
    let none = packs::load(&shipped(), &[], None).unwrap().hash();
    let uk = packs::load(&shipped(), &ids(&["uk"]), None).unwrap().hash();
    let au = packs::load(&shipped(), &ids(&["au"]), None).unwrap().hash();
    assert_eq!(none.len(), 64);
    assert!(none != uk && uk != au && none != au);
    assert_eq!(
        uk,
        packs::load(&shipped(), &ids(&["uk"]), None).unwrap().hash()
    );
}

#[test]
fn a_pack_that_breaks_a_rule_does_not_load() {
    let cases: [(&str, &str, &str, &str); 7] = [
        (
            "t",
            MANIFEST,
            &PHONE.replace("= 44", "= 0"),
            "outside 1 to 999",
        ),
        (
            "t",
            MANIFEST,
            &PHONE.replace("= 44", "= 1000"),
            "outside 1 to 999",
        ),
        // A vector the code cannot satisfy: the local number is +61 under code 61, not +44.
        ("t", MANIFEST, &PHONE.replace("= 44", "= 61"), "must match"),
        // Vectors are required.
        ("t", MANIFEST, "calling_code = 44\n", "vector"),
        // A key the schema does not know.
        (
            "t",
            MANIFEST,
            &format!("{PHONE}extra = 1\n"),
            "unknown field",
        ),
        (
            "t",
            &MANIFEST.replace("id = \"t\"", "id = \"other\""),
            PHONE,
            "says id",
        ),
        ("t", &MANIFEST.replace("none", "maybe"), PHONE, "reviewed"),
    ];
    for (id, manifest, phone, want) in cases {
        let dir = scratch(id, manifest, phone);
        let e = packs::load(&dir, &ids(&[id]), None).unwrap_err();
        assert!(e.contains(want), "wanted {want:?} in {e}");
    }
}

#[test]
fn a_differ_vector_that_matches_is_refused() {
    let phone = PHONE.replace("+61 7700 900123\"\n", "+44 7700 900123\"\n");
    let dir = scratch("t", MANIFEST, &phone);
    let e = packs::load(&dir, &ids(&["t"]), None).unwrap_err();
    assert!(e.contains("must differ"), "{e}");
}

#[test]
fn a_bad_id_or_a_missing_pack_is_refused_and_a_pack_is_not_enabled_twice() {
    for bad in ["../x", "UK", "", "a b"] {
        assert!(
            packs::load(&shipped(), &ids(&[bad]), None).is_err(),
            "{bad:?}"
        );
    }
    assert!(packs::load(&shipped(), &ids(&["nowhere"]), None).is_err());
    let e = packs::load(&shipped(), &ids(&["uk", "uk"]), None).unwrap_err();
    assert!(e.contains("twice"), "{e}");
}

#[test]
fn the_setting_is_a_comma_separated_list() {
    assert_eq!(parse_enabled(None), Vec::<String>::new());
    assert_eq!(parse_enabled(Some(" uk , ,au,")), ids(&["uk", "au"]));
    assert_eq!(parse_enabled(Some("")), Vec::<String>::new());
}

#[test]
fn the_code_decides_what_a_local_number_means() {
    // The same two records: a number written locally and the same written with +44. Under the
    // matcher's default (44) they match; read as an Australian number they do not.
    let (a, b) = (
        phone_patient("07700 900123"),
        phone_patient("+44 7700 900123"),
    );
    let score = |engine: &MatchingEngine| {
        engine
            .compare(&a, &b)
            .fields
            .iter()
            .find(|f| f.field == "telecom")
            .and_then(|f| f.similarity)
    };
    assert_eq!(score(&MatchingEngine::default()), Some(1.0));
    let au = MatchingEngine::new(electronic_patient_record_matcher::MatchConfig {
        default_calling_code: 61,
        ..Default::default()
    });
    assert_ne!(score(&au), Some(1.0));
}

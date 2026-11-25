//! DB-gated (AG-8, ID-1): how often the duplicate check even **looks at** the true record. The database retrieves at most
//! `CANDIDATE_LIMIT` patients by identifier, birth date or family name, and the matcher judges only those, so a duplicate
//! outside that set is missed whatever the matcher would have said.
//!
//! Synthetic people only, invented here by the author: it measures the kinds of difference it lists, **not** how often
//! each happens, and it is not a recall figure for any site. The table is in `spec/patient-identity/candidate-recall.md`.
//!
//! ```text
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test main candidate_recall:: -- --ignored --test-threads=1 --nocapture
//! ```

use electronic_patient_record::patients_store::{self, CANDIDATE_LIMIT};
use electronic_patient_record_matcher::{
    Decision, HumanName, Identifier, MatchingEngine, NATIONAL_NUMBER_SYSTEM, Patient,
};
use sqlx::{AssertSqlSafe, PgPool};

struct Scratch {
    db: PgPool,
    admin: PgPool,
    name: String,
}

impl Scratch {
    async fn new() -> Self {
        let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
        let (base, _) = url.rsplit_once('/').expect("a database URL");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("epr_scratch_{}_{}", std::process::id(), nanos);
        let admin = PgPool::connect(&url).await.expect("connect admin");
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&admin)
            .await
            .expect("create scratch db");
        let db = PgPool::connect(&format!("{base}/{name}"))
            .await
            .expect("connect scratch");
        electronic_patient_record::MIGRATOR
            .run(&db)
            .await
            .expect("migrate");
        Self { db, admin, name }
    }

    async fn finish(self) {
        self.db.close().await;
        let _ = sqlx::raw_sql(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {} WITH (FORCE)",
            self.name
        )))
        .execute(&self.admin)
        .await;
    }
}

fn person(family: &str, given: &str, birth: Option<&str>, number: Option<&str>) -> Patient {
    Patient {
        names: vec![HumanName {
            family: Some(family.into()),
            given: vec![given.into()],
        }],
        birth_date: birth.map(String::from),
        identifiers: number
            .map(|n| Identifier {
                system: NATIONAL_NUMBER_SYSTEM.into(),
                value: n.into(),
            })
            .into_iter()
            .collect(),
        ..Default::default()
    }
}

/// A kind of difference between two registrations of one person: the record on file, the new registration, whether the
/// database retrieves the first for the second, and what the matcher says of the two.
type Case = (&'static str, Patient, Patient, bool, Decision);

fn same_person() -> Vec<Case> {
    use Decision::{AutoLink, NonMatch, Review};
    let b = Some("1984-03-07");
    let n = |family, given, birth| person(family, given, birth, None);
    vec![
        (
            "identical",
            n("Okafor", "Adaeze", b),
            n("Okafor", "Adaeze", b),
            true,
            AutoLink,
        ),
        (
            "family name misspelt",
            n("Lindqvist", "Maja", b),
            n("Lindquist", "Maja", b),
            true,
            AutoLink,
        ),
        (
            "family name changed",
            n("Hughes", "Carys", b),
            n("Morgan", "Carys", b),
            true,
            Review,
        ),
        (
            "day and month swapped",
            n("Novak", "Petra", b),
            n("Novak", "Petra", Some("1984-07-03")),
            true,
            Review,
        ),
        (
            "misspelt, and day and month swapped",
            n("Svensson", "Erik", b),
            n("Svenson", "Erik", Some("1984-07-03")),
            false,
            Review,
        ),
        (
            "family name changed, and year one out",
            n("Baker", "Rhiannon", Some("1971-05-12")),
            n("Price", "Rhiannon", Some("1972-05-12")),
            false,
            NonMatch,
        ),
        (
            "misspelt, and only a birth year known",
            n("Petrov", "Ivan", Some("1960-01-09")),
            n("Petrof", "Ivan", Some("1960")),
            false,
            Review,
        ),
        (
            "misspelt, and no birth date known",
            n("Kowalski", "Jan", Some("1955-02-02")),
            n("Kowalsky", "Jan", None),
            false,
            Review,
        ),
        (
            "accent lost",
            n("M\u{fc}ller", "J\u{fc}rgen", b),
            n("Muller", "Jurgen", b),
            true,
            AutoLink,
        ),
        (
            "accent lost, and only a birth year known",
            n("G\u{f3}mez", "Luc\u{ed}a", Some("1990-10-10")),
            n("Gomez", "Lucia", Some("1990")),
            false,
            Review,
        ),
        (
            "another script",
            n("Шевченко", "Олена", b),
            n("Shevchenko", "Olena", b),
            true,
            NonMatch,
        ),
        (
            "another script, and day and month swapped",
            n("Мельник", "Андрій", Some("1979-04-11")),
            n("Melnyk", "Andriy", Some("1979-11-04")),
            false,
            NonMatch,
        ),
        (
            "family and given name swapped",
            n("Nguyen", "Thi", b),
            n("Thi", "Nguyen", b),
            true,
            Review,
        ),
        (
            "family and given name swapped, and day and month swapped",
            n("Tran", "Van", Some("1988-06-09")),
            n("Van", "Tran", Some("1988-09-06")),
            false,
            Review,
        ),
        (
            "half of a double family name",
            n("Garcia Lopez", "Ana", b),
            n("Garcia", "Ana", b),
            true,
            AutoLink,
        ),
        (
            "half of a double family name, and year one out",
            n("Costa Silva", "Rui", Some("1969-12-01")),
            n("Costa", "Rui", Some("1968-12-01")),
            false,
            Review,
        ),
        (
            "hyphen against space, and only a birth year known",
            n("Smith-Jones", "Ella", Some("2001-08-08")),
            n("Smith Jones", "Ella", Some("2001")),
            false,
            Review,
        ),
        (
            "same number, all else different",
            person("Evans", "Non", Some("1944-01-01"), Some("9434765919")),
            person("Vaughan", "Nonny", Some("1945-02-02"), Some("9434765919")),
            true,
            Review,
        ),
        (
            "same number written with spaces, and all else different",
            person("Rees", "Mair", Some("1939-09-09"), Some("9434765870")),
            person("Parry", "Mairwen", Some("1938-08-08"), Some("943 476 5870")),
            false,
            Review,
        ),
    ]
}

/// Two people who are not one, whom the matcher must never link by itself.
fn different_people() -> Vec<(&'static str, Patient, Patient)> {
    let n = |family, given, birth| person(family, given, birth, None);
    let b = Some("1992-11-30");
    vec![
        (
            "twins with unlike names",
            n("Abebe", "Hana", b),
            n("Abebe", "Dawit", b),
        ),
        (
            "twins with like names",
            n("Larsen", "Jon", b),
            n("Larsen", "Jan", b),
        ),
        (
            "twins, one letter apart",
            n("Osei", "Kofi", b),
            n("Osei", "Kofe", b),
        ),
        (
            "parent and child of one name",
            n("Da Silva", "Maria", Some("1955-03-03")),
            n("Da Silva", "Maria", Some("1985-03-03")),
        ),
        (
            "a common name, another birth date",
            n("Khan", "Ali", Some("1970-01-02")),
            n("Khan", "Ali", Some("1991-06-17")),
        ),
    ]
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn which_kinds_of_duplicate_are_retrieved_and_which_are_never_looked_at() {
    let s = Scratch::new().await;
    let engine = MatchingEngine::default();
    let cases = same_person();
    let mut ids = Vec::new();
    for (_, stored, ..) in &cases {
        ids.push(
            patients_store::create(&s.db, "t", None, stored)
                .await
                .unwrap()
                .id,
        );
    }
    let (mut retrieved, mut lost) = (0, 0);
    let mut wrong = Vec::new();
    for ((what, stored, again, seen, decision), id) in cases.iter().zip(&ids) {
        let found = patients_store::candidates(&s.db, again).await.unwrap();
        let now_seen = found.iter().any(|f| f.id == *id);
        let now_decided = engine.compare(stored, again).decision;
        println!(
            "| {what} | {} | {now_decided:?} |",
            if now_seen { "yes" } else { "**no**" }
        );
        retrieved += i32::from(now_seen);
        lost += i32::from(!now_seen && now_decided != Decision::NonMatch);
        if (now_seen, &now_decided) != (*seen, decision) {
            wrong.push(format!("{what}: retrieved {now_seen}, {now_decided:?}"));
        }
    }
    // Pinned as it is, misses included: a change here is a change in what registration can find, for better or worse.
    assert!(
        wrong.is_empty(),
        "the table has changed:\n{}",
        wrong.join("\n")
    );
    assert_eq!((cases.len(), retrieved, lost), (19, 9, 8));
    s.finish().await;
}

/// Pairs of two people that the matcher, today, would let be linked automatically (AG-13, OQ-AG8). Nothing in the service
/// links by itself (below), but the screen shows the decision and the matcher is a library others may use. A pair leaves
/// this list when the matcher is changed, and one may not join it.
const KNOWN_WRONG: [&str; 1] = ["twins, one letter apart"];

#[test]
fn the_pairs_of_two_people_that_the_matcher_would_link_are_the_known_ones() {
    let engine = MatchingEngine::default();
    let mut linked = Vec::new();
    for (what, a, b) in different_people() {
        let r = engine.compare(&a, &b);
        println!("| {what} | {:?} | {:.3} |", r.decision, r.score);
        if r.decision == Decision::AutoLink {
            linked.push(what);
        }
    }
    assert_eq!(linked, KNOWN_WRONG);
}

/// What keeps the list above from harm: no code of the service acts on `AutoLink`. Only a merge, which a person asks for
/// with a reason, reads it, and only to record what the matcher said.
#[test]
fn nothing_in_the_service_links_two_records_by_itself() {
    let mut named = Vec::new();
    let mut read = 0;
    let mut dirs = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                read += 1;
                let text = std::fs::read_to_string(&path).unwrap();
                let code = text.split("#[cfg(test)]").next().unwrap();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                if code.contains("AutoLink") {
                    named.push(format!("AutoLink in {name}"));
                }
                if code.contains("links::merge(") || code.contains("store::merge(") {
                    named.push(format!("merge called in {name}"));
                }
            }
        }
    }
    assert!(read > 60, "the census read too little: {read}");
    named.sort();
    assert_eq!(
        named,
        ["AutoLink in links.rs", "merge called in links.rs"],
        "look at what links two records, and why"
    );
}

/// More people of one family name than the limit: strong evidence is kept, and a duplicate that shares only the name is
/// left to chance, because the last order is by a random id. `candidates_cut` says the list was cut.
#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_common_family_name_fills_the_list_and_only_strong_evidence_is_sure_of_a_place() {
    let s = Scratch::new().await;
    const PEOPLE: usize = 300;
    let mut ids = Vec::new();
    for i in 0..PEOPLE {
        let birth = format!("19{:02}-{:02}-{:02}", i % 100, 1 + i / 100, 13 + i % 10);
        let p = person("Smith", &format!("Person{i}"), Some(&birth), None);
        ids.push((
            patients_store::create(&s.db, "t", None, &p)
                .await
                .unwrap()
                .id,
            birth,
            i,
        ));
    }
    let (mut weak, mut strong) = (0, 0);
    for (id, birth, i) in &ids {
        // The same person with a birth date nobody has.
        let again = person("Smith", &format!("Person{i}"), Some("2020-01-01"), None);
        let (found, cut) = patients_store::candidates_ranked(&s.db, &again)
            .await
            .unwrap();
        assert!(cut, "it must say that the list was cut");
        weak += usize::from(found.iter().any(|f| f.id == *id));
        // The same person with the birth date on file.
        let again = person("Smith", &format!("Person{i}"), Some(birth), None);
        let found = patients_store::candidates(&s.db, &again).await.unwrap();
        strong += usize::from(found.iter().any(|f| f.id == *id));
    }
    println!("of {PEOPLE}: name alone {weak}, name and birth date {strong}");
    assert_eq!(strong, PEOPLE);
    assert_eq!(
        weak,
        usize::try_from(CANDIDATE_LIMIT).unwrap(),
        "each place is taken once"
    );
    s.finish().await;
}

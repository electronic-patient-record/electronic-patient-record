//! DB-gated: the MLLP listener end to end over a real TCP socket and a real database. A message is
//! read, matched, applied or refused, audited, and answered with `AA`, `AE` or `AR`.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DB_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test mllp_feed -- --ignored --test-threads=1
//! ```

use electronic_patient_record::adt_feed::{KeptAnswer, control_digest};
use electronic_patient_record::death::{Actor, Answer, Occurred, Status, Step};
use electronic_patient_record::death_store;
use electronic_patient_record::mllp::{self, CR, Config, FS, VT};
use electronic_patient_record::patients_store::{self, Search};
use sqlx::{AssertSqlSafe, PgPool, Row};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// The agent every listener here runs as.
const FEED_AGENT: &str = "mllp:test-feed";

async fn db() -> PgPool {
    let url = std::env::var("DB_URL").expect("DB_URL must be set for --ignored tests");
    // A pool of ONE connection, like the development default: code that holds a transaction and
    // then asks the pool for another connection hangs here instead of passing by luck.
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&url)
        .await
        .expect("connect");
    electronic_patient_record::MIGRATOR
        .run(&db)
        .await
        .expect("migrate");
    db
}

/// A well-mixed number, different on every call and every run. (The clock alone is no good: it
/// ticks in whole microseconds, so a remainder taken from it takes only a few distinct values.)
fn mixed() -> u64 {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut z = u64::try_from(nanos & u128::from(u64::MAX)).unwrap()
        ^ COUNTER
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A name that no other test run has used and that is not close to one either: the duplicate rule
/// (rightly) treats look-alike names with the same birth date as one person, and a test database
/// fills up over many runs. Random-looking letters first, then the label.
fn tag(label: &str) -> String {
    let mut letters = String::from("Zz");
    for _ in 0..12 {
        letters.push(char::from(b'a' + u8::try_from(mixed() % 26).unwrap()));
    }
    format!("{letters}{label}")
}

/// A birth date unlikely to repeat, so look-alike test patients are not (rightly) taken for one
/// person by the duplicate rule.
fn dob() -> String {
    let n = mixed();
    format!(
        "{}{:02}{:02}",
        1930 + n % 90,
        1 + n / 90 % 12,
        1 + n / 1080 % 28
    )
}

/// `19800314` as `1980-03-14`.
fn iso(d: &str) -> String {
    format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..])
}

async fn start(db: &PgPool, tweak: impl FnOnce(&mut Config)) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut config = Config::local(addr);
    config.agent = "mllp:test-feed".into();
    tweak(&mut config);
    tokio::spawn(mllp::serve(listener, db.clone(), config));
    addr
}

/// The given name `adt` uses for a family name.
fn given_of(family: &str) -> String {
    let given: String = family.chars().skip(2).take(6).collect();
    if given.is_empty() {
        "Ann".into()
    } else {
        given
    }
}

fn adt(event: &str, control: &str, mrn: &str, family: &str, dob: &str, extra_pid: &str) -> String {
    // The given name follows the family name, so two test people share neither: the same given
    // name, sex and birth date alone is (rightly) a possible duplicate, and a test database fills
    // up over many runs.
    let given = given_of(family);
    format!(
        "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261021090000||ADT^{event}^ADT_A05|{control}|P|2.5\rEVN|{event}|20261021090000\rPID|1||{mrn}^^^LOCAL||{family}^{given}||{dob}|F{extra_pid}\r"
    )
}

/// Read one framed answer.
async fn read_ack<S: tokio::io::AsyncRead + Unpin>(stream: &mut S) -> Option<String> {
    let mut got = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut byte)).await {
            Ok(Ok(1)) => {
                got.push(byte[0]);
                if got.len() >= 2 && got[got.len() - 2] == FS && got[got.len() - 1] == CR {
                    let text = String::from_utf8(got[1..got.len() - 2].to_vec()).unwrap();
                    return Some(text);
                }
            }
            _ => return None,
        }
    }
}

async fn send<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    stream: &mut S,
    message: &str,
) -> Option<String> {
    stream
        .write_all(&mllp::frame(message.as_bytes()))
        .await
        .unwrap();
    read_ack(stream).await
}

fn code(ack: &str) -> &str {
    ack.split('\r')
        .find(|s| s.starts_with("MSA|"))
        .and_then(|s| s.split('|').nth(1))
        .unwrap_or("?")
}

async fn by_mrn(db: &PgPool, mrn: &str) -> Vec<patients_store::Stored> {
    patients_store::search(
        db,
        &Search {
            identifier: Some(format!("urn:hl7v2:LOCAL|{mrn}")),
            ..Search::default()
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_new_patient_is_created_a_repeat_changes_nothing_and_a_correction_is_a_new_version() {
    let db = db().await;
    let addr = start(&db, |_| {}).await;
    let (mrn, family, d) = (tag("mrn"), tag("Feed"), dob());
    let mut s = TcpStream::connect(addr).await.unwrap();

    let ack = send(&mut s, &adt("A28", "M1", &mrn, &family, &d, ""))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    assert!(ack.contains("MSA|AA|M1"), "{ack}");
    let made = by_mrn(&db, &mrn).await;
    assert_eq!(made.len(), 1);
    assert_eq!(
        (made[0].version, made[0].author.as_str()),
        (1, "mllp:test-feed")
    );

    // The same message again: acknowledged, nothing written.
    let again = send(&mut s, &adt("A28", "M2", &mrn, &family, &d, ""))
        .await
        .unwrap();
    assert_eq!(code(&again), "AA");
    assert_eq!(by_mrn(&db, &mrn).await[0].version, 1);

    // A correction to the name and a phone number: version 2; the birth date and sex are kept.
    let fixed = adt("A08", "M3", &mrn, &family, &d, "|||||01632 960001");
    let ack = send(
        &mut s,
        &fixed.replace(&format!("^{}|", given_of(&family)), "^Annie|"),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    let now = by_mrn(&db, &mrn).await;
    assert_eq!(now[0].version, 2);
    assert_eq!(now[0].patient.names[0].given, ["Annie"]);
    assert_eq!(now[0].patient.birth_date.as_deref(), Some(iso(&d).as_str()));

    // The audit trail: who (the configured agent), from where, what happened; no message content.
    let rows = sqlx::query(
        "SELECT action::text AS a, outcome, agent, network_address, detail::text AS d
             FROM audit_events WHERE lower(patient_ref) = $1 ORDER BY id",
    )
    .bind(format!("patient/{}", made[0].id))
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(rows.len(), 3);
    for r in &rows {
        assert_eq!(r.try_get::<String, _>("agent").unwrap(), "mllp:test-feed");
        assert_eq!(
            r.try_get::<String, _>("network_address").unwrap(),
            "127.0.0.1"
        );
        let d: String = r.try_get("d").unwrap();
        assert!(
            !d.contains(&family) && !d.contains("960001") && !d.contains(&iso(&d)),
            "{d}"
        );
    }
    let results: Vec<String> = rows
        .iter()
        .map(|r| r.try_get::<String, _>("d").unwrap())
        .collect();
    assert!(
        results[0].contains("created")
            && results[1].contains("unchanged")
            && results[2].contains("updated")
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn conflicts_duplicates_unsupported_and_invalid_messages_are_refused_and_change_nothing() {
    let db = db().await;
    let addr = start(&db, |_| {}).await;
    let (mrn, family, d) = (tag("mrn"), tag("Hold"), dob());
    let other_dob = dob();
    let mut s = TcpStream::connect(addr).await.unwrap();
    assert_eq!(
        code(
            &send(&mut s, &adt("A28", "C1", &mrn, &family, &d, ""))
                .await
                .unwrap()
        ),
        "AA"
    );

    // A different birth date for a known number is for a person to correct.
    let ack = send(&mut s, &adt("A08", "C2", &mrn, &family, &other_dob, ""))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AR");
    assert!(ack.contains("birth date"), "{ack}");
    assert_eq!(by_mrn(&db, &mrn).await[0].version, 1);

    // The same person under a new number is refused rather than registered twice.
    let other = tag("mrn");
    let ack = send(&mut s, &adt("A28", "C3", &other, &family, &d, ""))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AR");
    assert!(ack.contains("may be the same person"), "{ack}");
    assert!(by_mrn(&db, &other).await.is_empty());

    // A death is stored as a report, holds contact and leaves the identity record alone (BV-2, H-046).
    let death = format!("{}20261104", "|".repeat(21));
    let patient_id = by_mrn(&db, &mrn).await[0].id.clone();
    let ack = send(&mut s, &adt("A08", "C7", &mrn, &family, &d, &death))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    assert_eq!(by_mrn(&db, &mrn).await[0].version, 1);
    let told = death_store::answer(&db, &patient_id).await.unwrap();
    assert!(told.holds_contact());
    assert!(matches!(
        told,
        Answer::Asserted {
            confirmed: false,
            ..
        }
    ));
    let reports = death_store::list(&db, &patient_id).await.unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(
        reports[0].assertion.reported_by,
        Actor::Feed(FEED_AGENT.into())
    );
    assert_eq!(reports[0].author, FEED_AGENT);

    // A source repeats PID-29 on every later message: that adds nothing.
    let ack = send(&mut s, &adt("A08", "C8", &mrn, &family, &d, &death))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    assert_eq!(death_store::list(&db, &patient_id).await.unwrap().len(), 1);

    // A date that cannot be stored still reports a death, with no date.
    let future = format!("{}29991231", "|".repeat(21));
    let ack = send(&mut s, &adt("A08", "C9", &mrn, &family, &d, &future))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    let reports = death_store::list(&db, &patient_id).await.unwrap();
    assert_eq!(reports.len(), 2);
    assert!(matches!(
        reports[0].assertion.occurred,
        Occurred::NotKnown { .. }
    ));

    // Once a person withdraws a report, the same report waits for a person rather than coming back.
    let by = Actor::Person("clinician-1".into());
    for r in &reports {
        let step = Step {
            to: Status::WithdrawnInError,
            by: by.clone(),
            reason: "the wrong record".into(),
        };
        death_store::advance(&db, &patient_id, &r.id, r.version, &step)
            .await
            .unwrap();
    }
    assert!(
        !death_store::answer(&db, &patient_id)
            .await
            .unwrap()
            .holds_contact()
    );
    let ack = send(&mut s, &adt("A08", "C10", &mrn, &family, &d, &death))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AR");
    assert!(ack.contains("withdrew as an error"), "{ack}");
    assert!(ack.contains("reports a death"), "{ack}");
    assert!(
        !death_store::answer(&db, &patient_id)
            .await
            .unwrap()
            .holds_contact()
    );

    // A later, different date from the same feed is a new report.
    let other_date = format!("{}20261105", "|".repeat(21));
    let ack = send(&mut s, &adt("A08", "C11", &mrn, &family, &d, &other_date))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    assert!(
        death_store::answer(&db, &patient_id)
            .await
            .unwrap()
            .holds_contact()
    );

    // A death for nobody on record waits for a person: it does not register them.
    let unknown = tag("mrn");
    let unknown_family = tag("Nobody");
    let ack = send(
        &mut s,
        &adt("A28", "C12", &unknown, &unknown_family, &dob(), &death),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AR");
    assert!(ack.contains("reports a death"), "{ack}");
    assert!(by_mrn(&db, &unknown).await.is_empty());

    // A message type this adapter does not handle is rejected, and so is one with no usable patient: both are kept
    // for a person, so both are `AR` (IH-1).
    let oru =
        adt("A28", "C4", &tag("m"), &family, &d, "").replace("ADT^A28^ADT_A05", "ORU^R01^ORU_R01");
    assert_eq!(code(&send(&mut s, &oru).await.unwrap()), "AR");
    let nameless = adt("A28", "C5", &tag("m"), "", &d, "");
    assert_eq!(code(&send(&mut s, &nameless).await.unwrap()), "AR");
    let bad_number = adt("A28", "C6", &tag("m"), &tag("Bad"), &d, "")
        .replace("PID|1||", "PID|1||9434765910^^^NHS^NH~");
    assert_eq!(code(&send(&mut s, &bad_number).await.unwrap()), "AR");

    // Every refusal is in the trail with outcome 4 and no content.
    let refused = sqlx::query("SELECT count(*) AS n FROM audit_events
             WHERE agent = 'mllp:test-feed' AND outcome = 4 AND detail->>'control_id' = ANY($1::text[])")
.bind(["C2", "C3", "C4", "C5", "C6"].map(|c| control_digest(FEED_AGENT, c)).to_vec())
.fetch_optional(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(refused.try_get::<i64, _>("n").unwrap() >= 5);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn pipelined_messages_are_answered_in_order_and_bad_streams_are_closed() {
    let db = db().await;
    let addr = start(&db, |_| {}).await;
    let mut s = TcpStream::connect(addr).await.unwrap();
    let (m1, m2) = (tag("p1"), tag("p2"));
    let both = [
        mllp::frame(adt("A28", "P1", &m1, &tag("Pipe"), &dob(), "").as_bytes()),
        mllp::frame(adt("A28", "P2", &m2, &tag("Pipe"), &dob(), "").as_bytes()),
    ]
    .concat();
    s.write_all(&both).await.unwrap();
    let (a1, a2) = (
        read_ack(&mut s).await.unwrap(),
        read_ack(&mut s).await.unwrap(),
    );
    assert!(
        a1.contains("MSA|AA|P1") && a2.contains("MSA|AA|P2"),
        "{a1} / {a2}"
    );

    // Not MLLP: closed without an answer.
    let mut junk = TcpStream::connect(addr).await.unwrap();
    junk.write_all(b"GET / HTTP/1.1\r\n\r\n").await.unwrap();
    assert_eq!(read_ack(&mut junk).await, None);
    // A frame that is not HL7: closed.
    let mut text = TcpStream::connect(addr).await.unwrap();
    text.write_all(&mllp::frame(b"hello, not a message"))
        .await
        .unwrap();
    assert_eq!(read_ack(&mut text).await, None);
    // Not UTF-8: closed.
    let mut bin = TcpStream::connect(addr).await.unwrap();
    bin.write_all(&mllp::frame(&[0xFF, 0xFE, 0xFD]))
        .await
        .unwrap();
    assert_eq!(read_ack(&mut bin).await, None);
    // The listener is still serving.
    let mut again = TcpStream::connect(addr).await.unwrap();
    let ack = send(
        &mut again,
        &adt("A28", "P3", &tag("p3"), &tag("Pipe"), &dob(), ""),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA");
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_connection_limit_and_the_idle_timeout_hold() {
    let db = db().await;
    let addr = start(&db, |c| {
        c.max_connections = 1;
        c.idle = Duration::from_secs(1);
    })
    .await;
    // One connection is served; a second is closed at once while the first is open.
    let mut first = TcpStream::connect(addr).await.unwrap();
    let ack = send(
        &mut first,
        &adt("A28", "L1", &tag("l"), &tag("Limit"), &dob(), ""),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA");
    let mut second = TcpStream::connect(addr).await.unwrap();
    let _ = second
        .write_all(&mllp::frame(
            adt("A28", "L2", &tag("l"), &tag("Limit"), &dob(), "").as_bytes(),
        ))
        .await;
    assert_eq!(
        read_ack(&mut second).await,
        None,
        "the second connection must not be served"
    );
    // The first goes quiet and is closed by the idle timeout, freeing the slot.
    let mut one = [0u8; 1];
    let closed = tokio::time::timeout(Duration::from_secs(5), first.read(&mut one)).await;
    assert!(matches!(closed, Ok(Ok(0)) | Ok(Err(_))), "{closed:?}");
    let mut third = TcpStream::connect(addr).await.unwrap();
    let ack = send(
        &mut third,
        &adt("A28", "L3", &tag("l"), &tag("Limit"), &dob(), ""),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA");
    let _ = VT; // framing constants are exercised by the helpers
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_refused_message_waits_in_the_queue_until_a_person_closes_it_once() {
    use electronic_patient_record::feed_queue::{self as queue, Outcome};
    use electronic_patient_record::patients_store::StoreError;
    let db = db().await;
    let addr = start(&db, |_| {}).await;
    let (mrn, family, d) = (tag("mrn"), tag("Queue"), dob());
    let waiting = queue::count_open(&db).await.unwrap();
    let mut s = TcpStream::connect(addr).await.unwrap();
    let first_control = tag("A");
    let first = send(&mut s, &adt("A28", &first_control, &mrn, &family, &d, ""))
        .await
        .unwrap();
    assert_eq!(code(&first), "AA", "{first}");
    // A different birth date is refused, and so is a type the feed does not handle.
    let control = tag("Q");
    let refused = adt("A08", &control, &mrn, &family, &dob(), "");
    assert_eq!(code(&send(&mut s, &refused).await.unwrap()), "AR");
    let oru_control = tag("R");
    let oru = adt("A28", &oru_control, &tag("m"), &family, &d, "")
        .replace("ADT^A28^ADT_A05", "ORU^R01^ORU_R01");
    assert_eq!(code(&send(&mut s, &oru).await.unwrap()), "AR");
    // The listing is capped and oldest-first, and a test database holds many old entries, so find
    // these by control id in the table.
    let id_of = |control: String| {
        let db = db.clone();
        async move {
            sqlx::query("SELECT refusal_id::text AS id FROM feed_refusals WHERE control_id = $1")
                .bind(control_digest(FEED_AGENT, &control))
                .fetch_optional(&db)
                .await
                .unwrap()
                .map(|r| r.try_get::<String, _>("id").unwrap())
        }
    };
    // An accepted message is not queued.
    assert_eq!(id_of(first_control).await, None);
    let id = id_of(control.clone())
        .await
        .expect("the refused message is queued");
    let oru_id = id_of(oru_control.clone()).await.expect("so is the other");

    assert_eq!(queue::count_open(&db).await.unwrap(), waiting + 2);
    // A listing carries no message text.
    for e in queue::list_open(&db).await.unwrap().rows {
        assert_eq!(e.message, None);
        assert!(e.resolution.is_none());
    }
    let entry = queue::get(&db, &id).await.unwrap();
    assert_eq!(
        (entry.kind.as_str(), entry.event.as_str()),
        ("conflict", "A08")
    );
    assert!(entry.reasons.iter().any(|r| r.contains("birth date")));
    assert_eq!(entry.agent, "mllp:test-feed");
    // The queue keeps the control id only as a fingerprint (IH-3); the message text has the real one.
    assert_eq!(entry.control_id, control_digest(FEED_AGENT, &control));
    assert_ne!(entry.control_id, control);
    assert_eq!(queue::get(&db, &oru_id).await.unwrap().kind, "unsupported");

    // One entry shows the message as received.
    let full = queue::get(&db, &entry.id).await.unwrap();
    assert_eq!(full.message.as_deref(), Some(refused.as_str()));
    assert_eq!(full.resolution, None);
    assert_eq!(
        queue::get(&db, "not-a-uuid").await,
        Err(StoreError::NotFound)
    );

    // Closing needs a reason, happens once, and takes it off the open list.
    assert!(matches!(
        queue::resolve(&db, &entry.id, "admin", Outcome::Handled, "  ").await,
        Err(StoreError::BadQuery(_))
    ));
    let closed = queue::resolve(
        &db,
        &entry.id,
        "admin",
        Outcome::Handled,
        "corrected by hand",
    )
    .await
    .unwrap();
    let r = closed.resolution.unwrap();
    assert_eq!(
        (r.resolved_by.as_str(), r.outcome.as_str()),
        ("admin", "handled")
    );
    assert!(matches!(
        queue::resolve(&db, &entry.id, "other", Outcome::Dismissed, "again").await,
        Err(StoreError::Conflict { .. })
    ));
    assert_eq!(queue::count_open(&db).await.unwrap(), waiting + 1);
    let still_open = sqlx::query(
        "SELECT count(*) AS n FROM feed_refusals f
             LEFT JOIN feed_refusal_resolutions r USING (refusal_id)
             WHERE r.refusal_id IS NULL AND f.refusal_id::text = $1",
    )
    .bind(entry.id.clone())
    .fetch_optional(&db)
    .await
    .unwrap()
    .unwrap();
    assert_eq!(still_open.try_get::<i64, _>("n").unwrap(), 0);
    // The closed one is still readable, with who closed it.
    assert_eq!(
        queue::get(&db, &entry.id)
            .await
            .unwrap()
            .resolution
            .unwrap()
            .resolved_by,
        "admin"
    );

    // Nothing in either table can be changed or removed.
    for sql in [
        format!(
            "UPDATE feed_refusals SET kind = 'conflict' WHERE refusal_id = '{}'",
            entry.id
        ),
        format!(
            "DELETE FROM feed_refusals WHERE refusal_id = '{}'",
            entry.id
        ),
        format!(
            "DELETE FROM feed_refusal_resolutions WHERE refusal_id = '{}'",
            entry.id
        ),
        "TRUNCATE feed_refusals CASCADE".to_string(),
    ] {
        let e = sqlx::raw_sql(AssertSqlSafe(sql.to_string()))
            .execute(&db)
            .await
            .unwrap_err()
            .to_string();
        assert!(e.contains("append-only"), "{sql}: {e}");
    }
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_refusal_list_longer_than_the_list_limit_says_it_was_cut_short() {
    // PR-1 (hazard H-044): oldest first and capped, so the newest refusal is the one hidden. Done in a
    // transaction that is rolled back, so the shared test database stays as it was.
    use electronic_patient_record::feed_queue::{self as queue, Kind, New};
    let db = db().await;
    let mut txn = db.begin().await.unwrap();
    let mut open = queue::count_open(&mut *txn).await.unwrap();
    while open < i64::try_from(queue::LIST_LIMIT).unwrap() + 1 {
        // A distinct control id each time: the same message again would be a resend, which adds no row (IH-1).
        let control = format!("c{open}");
        queue::record(
            &mut *txn,
            New {
                agent: "clinic-feed",
                peer: None,
                event: "A01",
                control_id: &control,
                kind: Kind::Unsupported,
                reasons: &[],
                message: "MSH|^~\\&|",
            },
        )
        .await
        .unwrap();
        open += 1;
    }
    let page = queue::list_open(&mut *txn).await.unwrap();
    assert_eq!(page.len() as u64, queue::LIST_LIMIT);
    assert!(
        page.more,
        "{open} are open but the list does not say it is cut"
    );
    txn.rollback().await.unwrap();
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_kept_refusal_is_answered_ar_and_a_resend_adds_no_row() {
    // IH-1: a sender that resends a refused message gets the same answer each time, and the work queue keeps one entry.
    let db = db().await;
    let addr = start(&db, |_| {}).await;
    let (mrn, family, d) = (tag("mrn"), tag("Resend"), dob());
    let mut s = TcpStream::connect(addr).await.unwrap();
    assert_eq!(
        code(
            &send(&mut s, &adt("A28", &tag("K"), &mrn, &family, &d, ""))
                .await
                .unwrap()
        ),
        "AA"
    );
    let control = tag("K");
    let refused = adt("A08", &control, &mrn, &family, &dob(), "");
    let mut answers = Vec::new();
    for _ in 0..3 {
        answers.push(send(&mut s, &refused).await.unwrap());
    }
    for ack in &answers {
        assert_eq!(code(ack), "AR", "{ack}");
        assert!(
            ack.contains("kept for review by a person; do not resend"),
            "{ack}"
        );
        assert!(ack.contains("birth date"), "{ack}");
    }
    let rows = |control: String| {
        let db = db.clone();
        async move {
            sqlx::query("SELECT count(*) AS n FROM feed_refusals WHERE control_id = $1")
                .bind(control_digest(FEED_AGENT, &control))
                .fetch_one(&db)
                .await
                .unwrap()
                .try_get::<i64, _>("n")
                .unwrap()
        }
    };
    assert_eq!(
        rows(control.clone()).await,
        1,
        "three sends, one queue entry"
    );
    // Each answer is counted by feed, event and answer, with nothing from the message (IH-9).
    let counted = |text: &str| {
        text.lines()
            .find(|l| {
                l.starts_with(
                    r#"epr_feed_messages_total{sender="mllp:test-feed",event="A08",answer="AR"}"#,
                )
            })
            .and_then(|l| l.rsplit(' ').next())
            .and_then(|n| n.parse::<u64>().ok())
            .unwrap_or(0)
    };
    assert!(counted(&electronic_patient_record::metrics::render()) >= 3);

    // The same control id with different text is a different message, and is kept on its own.
    let changed = adt("A08", &control, &mrn, &family, &dob(), "");
    assert_eq!(code(&send(&mut s, &changed).await.unwrap()), "AR");
    assert_eq!(rows(control.clone()).await, 2);

    // Once a person closes the entry, a resend is new work.
    let id: String = sqlx::query(
        "SELECT refusal_id::text AS id FROM feed_refusals WHERE control_id = $1 AND message = $2",
    )
    .bind(control_digest(FEED_AGENT, &control))
    .bind(&refused)
    .fetch_one(&db)
    .await
    .unwrap()
    .try_get("id")
    .unwrap();
    electronic_patient_record::feed_queue::resolve(
        &db,
        &id,
        "admin",
        electronic_patient_record::feed_queue::Outcome::Dismissed,
        "seen",
    )
    .await
    .unwrap();
    assert_eq!(code(&send(&mut s, &refused).await.unwrap()), "AR");
    assert_eq!(rows(control).await, 3);
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_sender_that_needs_ae_for_kept_refusals_can_have_it_but_a_type_not_handled_stays_ar() {
    let db = db().await;
    let addr = start(&db, |c| c.kept_answer = KeptAnswer::Error).await;
    let (mrn, family, d) = (tag("mrn"), tag("Kept"), dob());
    let mut s = TcpStream::connect(addr).await.unwrap();
    assert_eq!(
        code(
            &send(&mut s, &adt("A28", &tag("E"), &mrn, &family, &d, ""))
                .await
                .unwrap()
        ),
        "AA"
    );
    let ack = send(&mut s, &adt("A08", &tag("E"), &mrn, &family, &dob(), ""))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AE", "{ack}");
    assert!(ack.contains("kept for review by a person"), "{ack}");
    let oru = adt("A28", &tag("E"), &tag("m"), &family, &d, "")
        .replace("ADT^A28^ADT_A05", "ORU^R01^ORU_R01");
    assert_eq!(code(&send(&mut s, &oru).await.unwrap()), "AR");
}

#[tokio::test]
async fn a_refusal_that_cannot_be_kept_is_answered_ae_so_the_sender_tries_again() {
    // No database: nothing can be read or kept, so a resend is exactly what is wanted.
    let db = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap();
    let addr = start(&db, |_| {}).await;
    let mut s = TcpStream::connect(addr).await.unwrap();
    let oru = adt("A28", "T1", "m1", "Nobody", "19800314", "")
        .replace("ADT^A28^ADT_A05", "ORU^R01^ORU_R01");
    let ack = send(&mut s, &oru).await.unwrap();
    assert_eq!(
        code(&ack),
        "AR",
        "a type not handled is AR even when it cannot be kept: {ack}"
    );
    assert!(!ack.contains("kept for review"), "{ack}");
    let ack = send(&mut s, &adt("A28", "T2", "m2", "Nobody", "19800314", ""))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AE", "{ack}");
    assert!(!ack.contains("kept for review"), "{ack}");
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn the_plain_control_id_is_kept_only_in_the_stored_message() {
    // IH-3: a sender that builds MSH-10 from a patient's number must not have it copied into the audit trail, the
    // queue's columns or the record's version history. Only the stored message text, under the queue's access rule,
    // holds it, and the acknowledgement echoes it to the sender.
    let db = db().await;
    let addr = start(&db, |_| {}).await;
    let (mrn, family, d) = (tag("mrn"), tag("Plain"), dob());
    let accepted = format!("{}-9434765919", tag("P"));
    let refused = format!("{}-9434765919", tag("P"));
    let mut s = TcpStream::connect(addr).await.unwrap();
    let ack = send(&mut s, &adt("A28", &accepted, &mrn, &family, &d, ""))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    assert!(
        ack.contains(&format!("MSA|AA|{accepted}")),
        "the sender's own id is echoed: {ack}"
    );
    let message = adt("A08", &refused, &mrn, &family, &dob(), "");
    assert_eq!(code(&send(&mut s, &message).await.unwrap()), "AR");

    let count = |sql: &'static str, needle: String| {
        let db = db.clone();
        async move {
            sqlx::query(AssertSqlSafe(sql))
                .bind(format!("%{needle}%"))
                .fetch_one(&db)
                .await
                .unwrap()
                .try_get::<i64, _>("n")
                .unwrap()
        }
    };
    for plain in [&accepted, &refused] {
        assert_eq!(
            count(
                "SELECT count(*) AS n FROM audit_events WHERE detail::text LIKE $1",
                plain.clone()
            )
            .await,
            0,
            "an audit row holds {plain}"
        );
        assert_eq!(
            count(
                "SELECT count(*) AS n FROM patient_versions WHERE reason LIKE $1",
                plain.clone()
            )
            .await,
            0,
            "a version reason holds {plain}"
        );
        assert_eq!(
            count(
                "SELECT count(*) AS n FROM feed_refusals WHERE control_id LIKE $1 OR reasons::text LIKE $1",
                plain.clone()
            )
            .await,
            0,
            "a queue column holds {plain}"
        );
    }
    // The fingerprint is what joins them.
    assert_eq!(
        count(
            "SELECT count(*) AS n FROM audit_events WHERE detail::text LIKE $1",
            control_digest(FEED_AGENT, &accepted)
        )
        .await,
        1
    );
    assert_eq!(
        count(
            "SELECT count(*) AS n FROM feed_refusals WHERE message LIKE $1",
            refused.clone()
        )
        .await,
        1,
        "the stored message keeps the text as received"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn no_message_value_reaches_an_audit_row_the_queue_a_version_reason_a_metric_or_a_log_line() {
    // IH-4, end to end on the real path: the marker in names, address, phone and other fields, in a message that is
    // applied, one that is refused and kept, and one of a type not handled. The patient record itself holds the
    // names it was given, and the stored message holds the text; nothing else may.
    use electronic_patient_record::adt_feed::{self, FeedIdentity};
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Lines(Arc<Mutex<Vec<u8>>>);
    impl Write for Lines {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let lines = Lines::default();
    let writer = lines.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let db = db().await;
    let marker = tag("QzMark");
    let who = FeedIdentity {
        agent: FEED_AGENT.into(),
        peer: Some("127.0.0.1".into()),
        kept_answer: KeptAnswer::Reject,
    };
    let mrn = tag("mrn");
    let pid = |dob: &str| {
        format!(
            "PID|1||{mrn}^^^LOCAL||{marker}^{marker}||{dob}|{marker}|||{marker} St^^{marker}^^{marker}||{marker}|{marker}||||{marker}"
        )
    };
    let msh = |event: &str, structure: &str, control: &str| {
        format!(
            "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261117090000||ADT^{event}^{structure}|{control}|P|2.5\rEVN|{event}|20261117090000"
        )
    };
    let applied = format!("{}\r{}", msh("A28", "ADT_A05", &tag("V")), pid("19800314"));
    let refused = format!("{}\r{}", msh("A08", "ADT_A05", &tag("V")), pid("19800315"));
    let unsupported = format!(
        "{}\r{}",
        msh("R01", "ORU_R01", &tag("V")).replace("ADT^R01", "ORU^R01"),
        pid("19800314")
    );
    let before = electronic_patient_record::metrics::render();
    let mut acks = Vec::new();
    for text in [&applied, &refused, &unsupported] {
        acks.push(
            adt_feed::process(&db, text, &who, "ACK1", "20261117090001")
                .await
                .unwrap(),
        );
    }
    assert!(acks[0].contains("MSA|AA|"), "{}", acks[0]);
    assert!(acks[1].contains("MSA|AR|"), "{}", acks[1]);

    for ack in &acks {
        assert!(!ack.contains(&marker), "an acknowledgement: {ack}");
    }
    let logged = String::from_utf8(lines.0.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("ADT message applied"),
        "the capture works: {logged}"
    );
    assert!(!logged.contains(&marker), "a log line: {logged}");
    assert!(!electronic_patient_record::metrics::render().contains(&marker));
    assert!(!before.contains(&marker));
    let found = |sql: &'static str| {
        let db = db.clone();
        let needle = format!("%{marker}%");
        async move {
            sqlx::query(AssertSqlSafe(sql))
                .bind(needle)
                .fetch_one(&db)
                .await
                .unwrap()
                .try_get::<i64, _>("n")
                .unwrap()
        }
    };
    assert_eq!(found("SELECT count(*) AS n FROM audit_events WHERE detail::text LIKE $1 OR coalesce(network_address, '') LIKE $1").await, 0, "an audit row");
    assert_eq!(
        found("SELECT count(*) AS n FROM patient_versions WHERE coalesce(reason, '') LIKE $1")
            .await,
        0,
        "a version reason"
    );
    assert_eq!(
        found("SELECT count(*) AS n FROM feed_refusals WHERE reasons::text LIKE $1 OR control_id LIKE $1 OR event LIKE $1").await,
        0,
        "a queue column other than the message"
    );
    assert_eq!(
        found("SELECT count(*) AS n FROM feed_refusals WHERE message LIKE $1").await,
        2,
        "the two kept messages hold it"
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_late_update_does_not_undo_a_newer_one() {
    // IH-5: per feed and patient, an update older than the newest one applied waits for a person. The order is the
    // event time (EVN-2), or the message time (MSH-7) when there is none.
    let db = db().await;
    let addr = start(&db, |_| {}).await;
    let (mrn, family, d) = (tag("mrn"), tag("Late"), dob());
    let at = |m: String, t: &str| m.replace("20261021090000", t);
    let phone = |n: &str| format!("|||||{n}");
    let mut s = TcpStream::connect(addr).await.unwrap();
    let ack = send(
        &mut s,
        &at(
            adt("A28", &tag("L"), &mrn, &family, &d, ""),
            "20261117090000",
        ),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    // A newer update applies.
    let ack = send(
        &mut s,
        &at(
            adt("A08", &tag("L"), &mrn, &family, &d, &phone("01632960003")),
            "20261117110000",
        ),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    let version = by_mrn(&db, &mrn).await[0].version;
    assert_eq!(version, 2);

    // One sent at 10:00, arriving after the 11:00 one, is kept for a person and changes nothing.
    let late_control = tag("L");
    let late = at(
        adt(
            "A08",
            &late_control,
            &mrn,
            &family,
            &d,
            &phone("01632960002"),
        ),
        "20261117100000",
    );
    let ack = send(&mut s, &late).await.unwrap();
    assert_eq!(code(&ack), "AR", "{ack}");
    assert!(
        ack.contains("older than one this feed has already applied"),
        "{ack}"
    );
    assert!(!ack.contains("2026111710"), "no time in the answer: {ack}");
    assert_eq!(by_mrn(&db, &mrn).await[0].version, version);
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM feed_refusals WHERE control_id = $1")
        .bind(control_digest(FEED_AGENT, &late_control))
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(kept, 1);

    // The same time as the newest is not older: it applies.
    let ack = send(
        &mut s,
        &at(
            adt("A08", &tag("L"), &mrn, &family, &d, &phone("01632960004")),
            "20261117110000",
        ),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    assert_eq!(by_mrn(&db, &mrn).await[0].version, version + 1);

    // A message with no usable time applies as before, and its audit row says the time was not known.
    let timeless_control = tag("L");
    let timeless = adt(
        "A08",
        &timeless_control,
        &mrn,
        &family,
        &d,
        &phone("01632960005"),
    )
    .replace("|20261021090000||", "|||")
    .replace("EVN|A08|20261021090000", "EVN|A08|");
    let ack = send(&mut s, &timeless).await.unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    assert_eq!(by_mrn(&db, &mrn).await[0].version, version + 2);
    let known: Option<bool> = sqlx::query_scalar(
        "SELECT (detail->>'message_time_known')::boolean FROM audit_events WHERE detail->>'control_id' = $1",
    )
    .bind(control_digest(FEED_AGENT, &timeless_control))
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(known, Some(false));

    // Another feed's newest time does not hold this one back.
    let other_addr = start(&db, |c| c.agent = "mllp:other-feed".into()).await;
    let mut o = TcpStream::connect(other_addr).await.unwrap();
    let ack = send(
        &mut o,
        &at(
            adt("A08", &tag("L"), &mrn, &family, &d, &phone("01632960006")),
            "20261117080000",
        ),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
}

#[tokio::test]
#[ignore = "needs Postgres: set DB_URL"]
async fn a_feeds_health_is_judged_from_the_database_and_a_change_is_stored_once() {
    // IH-8: a feed of its own, so other tests' refusals do not count.
    use electronic_patient_record::feed_health::{self, State, Thresholds};
    let db = db().await;
    let agent = tag("mllp:health");
    let t = Thresholds::default();
    let utc = chrono_tz::UTC;
    // Never sent: unknown, and that is stored.
    let now = chrono::Utc::now();
    let (a, changed) = feed_health::check(&db, &agent, now, utc, &t).await.unwrap();
    assert_eq!((a.state, changed), (State::Unknown, true));

    let name = agent.clone();
    let addr = start(&db, move |c| c.agent = name).await;
    let mut s = TcpStream::connect(addr).await.unwrap();
    let ack = send(
        &mut s,
        &adt("A28", &tag("H"), &tag("mrn"), &tag("Well"), &dob(), ""),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    let now = chrono::Utc::now();
    let (a, changed) = feed_health::check(&db, &agent, now, utc, &t).await.unwrap();
    assert_eq!((a.state, changed), (State::Healthy, true), "{a:?}");
    // The same state again stores nothing, so a restart that judges again raises nothing.
    let (_, changed) = feed_health::check(&db, &agent, now, utc, &t).await.unwrap();
    assert!(!changed);
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM feed_alarm_states WHERE agent = $1")
        .bind(&agent)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(rows, 2);

    // A refusal makes it a warning; silence past every window makes it critical.
    let ack = send(
        &mut s,
        &adt("A08", &tag("H"), &tag("mrn"), &tag("Ill"), &dob(), "")
            .replace("PID|1||", "PID|1||9434765910^^^NHS^NH~"),
    )
    .await
    .unwrap();
    assert_eq!(code(&ack), "AR", "{ack}");
    let (a, _) = feed_health::check(&db, &agent, chrono::Utc::now(), utc, &t)
        .await
        .unwrap();
    assert_eq!(a.state, State::Warning, "{a:?}");
    let later = chrono::Utc::now() + chrono::Duration::days(2);
    let (a, changed) = feed_health::check(&db, &agent, later, utc, &t)
        .await
        .unwrap();
    assert_eq!((a.state, changed), (State::Critical, true), "{a:?}");
    assert!(a.reasons.contains(&feed_health::SILENT));
    assert_eq!(
        feed_health::last_state(&db, &agent)
            .await
            .unwrap()
            .map(|(s, _)| s),
        Some(State::Critical)
    );
    // The gauge says the same.
    assert!(
        electronic_patient_record::metrics::render()
            .contains(&format!("epr_feed_health{{sender=\"{agent}\"}} 3"))
    );
}

/// Certificates for the TLS tests, made with the `openssl` command in a fresh directory (a test CA, a server certificate
/// for 127.0.0.1 and a client certificate), so no private key is ever committed.
struct Certs {
    dir: std::path::PathBuf,
}

impl Certs {
    fn make() -> Certs {
        let dir = std::env::temp_dir().join(format!("epr-mllp-tls-{}", tag("t")));
        std::fs::create_dir_all(&dir).unwrap();
        let run = |args: &[&str]| {
            let out = std::process::Command::new("openssl")
                .args(args)
                .current_dir(&dir)
                .output()
                .expect("the TLS tests need the openssl command");
            assert!(
                out.status.success(),
                "openssl {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        let ec = [
            "-newkey",
            "ec",
            "-pkeyopt",
            "ec_paramgen_curve:P-256",
            "-nodes",
        ];
        run(&[
            &["req", "-x509"][..],
            &ec,
            &[
                "-keyout",
                "ca.key",
                "-out",
                "ca.pem",
                "-days",
                "30",
                "-subj",
                "/CN=test ca",
                "-addext",
                "basicConstraints=critical,CA:TRUE",
                "-addext",
                "keyUsage=critical,keyCertSign,cRLSign",
            ],
        ]
        .concat());
        for (name, usage, san) in [
            (
                "server",
                "serverAuth",
                "subjectAltName=IP:127.0.0.1,DNS:localhost",
            ),
            ("client", "clientAuth", "subjectAltName=DNS:test-sender"),
        ] {
            run(&[
                &["req"][..],
                &ec,
                &[
                    "-keyout",
                    &format!("{name}.key"),
                    "-out",
                    &format!("{name}.csr"),
                    "-subj",
                    &format!("/CN={name}"),
                ],
            ]
            .concat());
            std::fs::write(dir.join(format!("{name}.ext")), format!("basicConstraints=CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage={usage}\n{san}\n")).unwrap();
            run(&[
                "x509",
                "-req",
                "-in",
                &format!("{name}.csr"),
                "-CA",
                "ca.pem",
                "-CAkey",
                "ca.key",
                "-CAcreateserial",
                "-out",
                &format!("{name}.pem"),
                "-days",
                "30",
                "-extfile",
                &format!("{name}.ext"),
            ]);
        }
        Certs { dir }
    }
    fn path(&self, f: &str) -> String {
        self.dir.join(f).to_string_lossy().into_owned()
    }
}

impl Drop for Certs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A TLS client trusting the test CA, with the client certificate when `with_cert`.
fn tls_client(certs: &Certs, with_cert: bool) -> tokio_rustls::TlsConnector {
    use rustls_pki_types::pem::PemObject;
    use rustls_pki_types::{CertificateDer, PrivateKeyDer};
    let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
    let mut roots = rustls::RootCertStore::empty();
    for c in CertificateDer::pem_file_iter(certs.path("ca.pem")).unwrap() {
        roots.add(c.unwrap()).unwrap();
    }
    let b = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots);
    let config = if with_cert {
        let chain = CertificateDer::pem_file_iter(certs.path("client.pem"))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        b.with_client_auth_cert(
            chain,
            PrivateKeyDer::from_pem_file(certs.path("client.key")).unwrap(),
        )
        .unwrap()
    } else {
        b.with_no_client_auth()
    };
    tokio_rustls::TlsConnector::from(std::sync::Arc::new(config))
}

/// A listener with no database behind it: a type not handled is still answered `AR` (IH-1), which is all these tests
/// need to see that a message got through.
async fn start_tls(certs: &Certs, client_ca: bool) -> std::net::SocketAddr {
    let db = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap();
    let ca = certs.path("ca.pem");
    let tls = mllp::tls_config(
        &certs.path("server.pem"),
        &certs.path("server.key"),
        client_ca.then_some(ca.as_str()),
    )
    .unwrap();
    start(&db, move |c| c.tls = Some(tls)).await
}

fn oru() -> String {
    adt("A28", "T1", "m1", "Nobody", "19800314", "").replace("ADT^A28^ADT_A05", "ORU^R01^ORU_R01")
}

#[tokio::test]
async fn with_tls_a_message_is_answered_over_tls_and_plain_text_gets_nothing() {
    // IH-10.
    let certs = Certs::make();
    let addr = start_tls(&certs, false).await;
    let tcp = TcpStream::connect(addr).await.unwrap();
    let name = rustls_pki_types::ServerName::try_from("127.0.0.1").unwrap();
    let mut secure = tls_client(&certs, false)
        .connect(name, tcp)
        .await
        .expect("the handshake completes");
    let ack = send(&mut secure, &oru()).await.expect("an answer over TLS");
    assert_eq!(code(&ack), "AR", "{ack}");

    // The same port in plain text: no acknowledgement, ever.
    let mut plain = TcpStream::connect(addr).await.unwrap();
    plain
        .write_all(&mllp::frame(oru().as_bytes()))
        .await
        .unwrap();
    assert_eq!(read_ack(&mut plain).await, None);
}

#[tokio::test]
async fn with_a_client_ca_only_a_sender_with_a_certificate_is_served() {
    let certs = Certs::make();
    let addr = start_tls(&certs, true).await;
    let name = || rustls_pki_types::ServerName::try_from("127.0.0.1").unwrap();
    // Without a certificate the handshake fails, or the first read finds the connection closed.
    let tcp = TcpStream::connect(addr).await.unwrap();
    if let Ok(mut s) = tls_client(&certs, false).connect(name(), tcp).await {
        let _ = s.write_all(&mllp::frame(oru().as_bytes())).await;
        assert_eq!(
            read_ack(&mut s).await,
            None,
            "a sender with no certificate was served"
        );
    }
    let tcp = TcpStream::connect(addr).await.unwrap();
    let mut s = tls_client(&certs, true)
        .connect(name(), tcp)
        .await
        .expect("a sender with a certificate is let in");
    assert_eq!(code(&send(&mut s, &oru()).await.unwrap()), "AR");
}

#[test]
fn a_tls_setting_that_cannot_be_used_stops_the_start() {
    let certs = Certs::make();
    let empty = certs.path("empty.pem");
    std::fs::write(&empty, "\n").unwrap();
    let (cert, key, client_key, ca) = (
        certs.path("server.pem"),
        certs.path("server.key"),
        certs.path("client.key"),
        certs.path("ca.pem"),
    );
    let missing = certs.path("missing.pem");
    let bind = ("EPR_MLLP_BIND", "127.0.0.1:2575");
    let config = |pairs: Vec<(&str, &str)>| {
        let map: std::collections::HashMap<String, String> = std::iter::once(bind)
            .chain(pairs)
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(move |k| map.get(k).cloned())
    };
    assert!(
        config(vec![]).unwrap().unwrap().tls.is_none(),
        "no TLS unless asked"
    );
    assert!(
        config(vec![
            ("EPR_MLLP_TLS_CERT_FILE", &cert),
            ("EPR_MLLP_TLS_KEY_FILE", &key)
        ])
        .unwrap()
        .unwrap()
        .tls
        .is_some()
    );
    assert!(
        config(vec![
            ("EPR_MLLP_TLS_CERT_FILE", &cert),
            ("EPR_MLLP_TLS_KEY_FILE", &key),
            ("EPR_MLLP_TLS_CLIENT_CA_FILE", &ca)
        ])
        .unwrap()
        .unwrap()
        .tls
        .is_some()
    );
    for (why, pairs) in [
        (
            "a certificate with no key",
            vec![("EPR_MLLP_TLS_CERT_FILE", cert.as_str())],
        ),
        (
            "a key with no certificate",
            vec![("EPR_MLLP_TLS_KEY_FILE", key.as_str())],
        ),
        (
            "a missing file",
            vec![
                ("EPR_MLLP_TLS_CERT_FILE", missing.as_str()),
                ("EPR_MLLP_TLS_KEY_FILE", key.as_str()),
            ],
        ),
        (
            "an empty certificate",
            vec![
                ("EPR_MLLP_TLS_CERT_FILE", empty.as_str()),
                ("EPR_MLLP_TLS_KEY_FILE", key.as_str()),
            ],
        ),
        (
            "an empty key",
            vec![
                ("EPR_MLLP_TLS_CERT_FILE", cert.as_str()),
                ("EPR_MLLP_TLS_KEY_FILE", empty.as_str()),
            ],
        ),
        (
            "a key that does not fit",
            vec![
                ("EPR_MLLP_TLS_CERT_FILE", cert.as_str()),
                ("EPR_MLLP_TLS_KEY_FILE", client_key.as_str()),
            ],
        ),
        (
            "a certificate file that is a key",
            vec![
                ("EPR_MLLP_TLS_CERT_FILE", key.as_str()),
                ("EPR_MLLP_TLS_KEY_FILE", key.as_str()),
            ],
        ),
        (
            "a client CA with no TLS",
            vec![("EPR_MLLP_TLS_CLIENT_CA_FILE", ca.as_str())],
        ),
        (
            "an empty client CA",
            vec![
                ("EPR_MLLP_TLS_CERT_FILE", cert.as_str()),
                ("EPR_MLLP_TLS_KEY_FILE", key.as_str()),
                ("EPR_MLLP_TLS_CLIENT_CA_FILE", empty.as_str()),
            ],
        ),
    ] {
        let got = config(pairs);
        assert!(got.is_err(), "{why} was accepted");
    }
}

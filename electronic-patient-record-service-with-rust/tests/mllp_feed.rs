//! DB-gated: the MLLP listener end to end over a real TCP socket and a real database. A message is
//! read, matched, applied or refused, audited, and answered with `AA`, `AE` or `AR`.
//!
//! ```text
//! TEST_DB_PORT=5543 podman compose -f compose.test.yaml up -d --wait
//! DATABASE_URL=postgres://loco:loco@localhost:5543/electronic_patient_record_test \
//!   cargo test --test mllp_feed -- --ignored --test-threads=1
//! ```

use electronic_patient_record::mllp::{self, CR, Config, FS, VT};
use electronic_patient_record::patients_store::{self, Search};
use electronic_patient_record_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for --ignored tests");
    // A pool of ONE connection, like the development default: code that holds a transaction and
    // then asks the pool for another connection hangs here instead of passing by luck.
    let mut options = sea_orm::ConnectOptions::new(url);
    options
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10));
    let db = Database::connect(options).await.expect("connect");
    Migrator::up(&db, None).await.expect("migrate");
    db
}

fn tag(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("Zz{label}{nanos}")
}

/// A birth date unlikely to repeat, so look-alike test patients are not (rightly) taken for one
/// person by the duplicate rule.
fn dob() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    format!(
        "19{:02}{:02}{:02}",
        30 + n % 60,
        1 + n / 60 % 12,
        1 + n / 720 % 28
    )
}

/// `19800314` as `1980-03-14`.
fn iso(d: &str) -> String {
    format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..])
}

async fn start(db: &DatabaseConnection, tweak: impl FnOnce(&mut Config)) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut config = Config::local(addr);
    config.agent = "mllp:test-feed".into();
    tweak(&mut config);
    tokio::spawn(mllp::serve(listener, db.clone(), config));
    addr
}

fn adt(event: &str, control: &str, mrn: &str, family: &str, dob: &str, extra_pid: &str) -> String {
    format!(
        "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261021090000||ADT^{event}^ADT_A05|{control}|P|2.5\rEVN|{event}|20261021090000\rPID|1||{mrn}^^^LOCAL||{family}^Ann||{dob}|F{extra_pid}\r"
    )
}

/// Read one framed answer.
async fn read_ack(stream: &mut TcpStream) -> Option<String> {
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

async fn send(stream: &mut TcpStream, message: &str) -> Option<String> {
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

async fn by_mrn(db: &DatabaseConnection, mrn: &str) -> Vec<patients_store::Stored> {
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
#[ignore = "needs Postgres: set DATABASE_URL"]
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
    let ack = send(&mut s, &fixed.replace("^Ann|", "^Annie|"))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AA", "{ack}");
    let now = by_mrn(&db, &mrn).await;
    assert_eq!(now[0].version, 2);
    assert_eq!(now[0].patient.names[0].given, ["Annie"]);
    assert_eq!(now[0].patient.birth_date.as_deref(), Some(iso(&d).as_str()));

    // The audit trail: who (the configured agent), from where, what happened; no message content.
    let rows = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT action::text AS a, outcome, agent, network_address, detail::text AS d
             FROM audit_events WHERE lower(patient_ref) = $1 ORDER BY id",
            [format!("patient/{}", made[0].id).into()],
        ))
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);
    for r in &rows {
        assert_eq!(r.try_get::<String>("", "agent").unwrap(), "mllp:test-feed");
        assert_eq!(
            r.try_get::<String>("", "network_address").unwrap(),
            "127.0.0.1"
        );
        let d: String = r.try_get("", "d").unwrap();
        assert!(
            !d.contains(&family) && !d.contains("960001") && !d.contains(&iso(&d)),
            "{d}"
        );
    }
    let results: Vec<String> = rows
        .iter()
        .map(|r| r.try_get::<String>("", "d").unwrap())
        .collect();
    assert!(
        results[0].contains("created")
            && results[1].contains("unchanged")
            && results[2].contains("updated")
    );
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
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
    assert_eq!(code(&ack), "AE");
    assert!(ack.contains("birth date"), "{ack}");
    assert_eq!(by_mrn(&db, &mrn).await[0].version, 1);

    // The same person under a new number is refused rather than registered twice.
    let other = tag("mrn");
    let ack = send(&mut s, &adt("A28", "C3", &other, &family, &d, ""))
        .await
        .unwrap();
    assert_eq!(code(&ack), "AE");
    assert!(ack.contains("may be the same person"), "{ack}");
    assert!(by_mrn(&db, &other).await.is_empty());

    // A message type this adapter does not handle is rejected, and one with no usable patient is an error.
    let oru =
        adt("A28", "C4", &tag("m"), &family, &d, "").replace("ADT^A28^ADT_A05", "ORU^R01^ORU_R01");
    assert_eq!(code(&send(&mut s, &oru).await.unwrap()), "AR");
    let nameless = adt("A28", "C5", &tag("m"), "", &d, "");
    assert_eq!(code(&send(&mut s, &nameless).await.unwrap()), "AE");
    let bad_nhs = adt("A28", "C6", &tag("m"), &tag("Bad"), &d, "")
        .replace("PID|1||", "PID|1||9434765910^^^NHS^NH~");
    assert_eq!(code(&send(&mut s, &bad_nhs).await.unwrap()), "AE");

    // Every refusal is in the trail with outcome 4 and no content.
    let refused = db
        .query_one_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT count(*) AS n FROM audit_events
             WHERE agent = 'mllp:test-feed' AND outcome = 4 AND detail->>'control_id' = ANY($1::text[])",
            [vec!["C2".to_string(), "C3".to_string(), "C4".to_string(), "C5".to_string(), "C6".to_string()].into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(refused.try_get::<i64>("", "n").unwrap() >= 5);
}

#[tokio::test]
#[ignore = "needs Postgres: set DATABASE_URL"]
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
#[ignore = "needs Postgres: set DATABASE_URL"]
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

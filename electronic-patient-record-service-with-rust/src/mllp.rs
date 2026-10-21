//! An MLLP listener for HL7 v2 ADT feeds (task OR-3).
//!
//! MLLP is a thin frame around an HL7 message on a TCP stream: `<VT> message <FS><CR>` (bytes
//! `0x0B`, then `0x1C 0x0D`). The listener reads frames, hands each message to
//! [`adt_feed::process`](crate::adt_feed::process) and writes the acknowledgement back in a frame.
//!
//! **MLLP has no authentication and no encryption.** The listener is therefore off unless
//! `EPR_MLLP_BIND` is set, refuses a non-loopback address unless `EPR_MLLP_ALLOW_REMOTE=true`, and
//! when it is allowed to listen beyond the machine it must sit behind a TLS terminator or a private
//! network that the deployer controls. Whatever the sender says about itself in `MSH` is not used to
//! decide anything: the audit agent is `EPR_MLLP_AGENT`, and the peer address is recorded.
//!
//! Limits: one frame is at most 1 MiB; a connection that is silent for `EPR_MLLP_IDLE_SECONDS` (60)
//! is closed; at most `EPR_MLLP_MAX_CONNECTIONS` (8) are served at once and the rest are closed on
//! accept. A byte stream that does not start a frame with `<VT>`, a frame that never ends, a stray
//! `<VT>` inside a frame and text that is not UTF-8 each close the connection: nothing is guessed.

use crate::adt_feed::{self, FeedIdentity};
use sea_orm::DatabaseConnection;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;

/// Start of block.
pub const VT: u8 = 0x0B;
/// End of block.
pub const FS: u8 = 0x1C;
/// Carriage return, after the end of block.
pub const CR: u8 = 0x0D;

/// The most bytes one message may take.
pub const MAX_FRAME: usize = 1024 * 1024;

/// Why a byte stream is not valid MLLP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// Data that does not begin with `<VT>`.
    NoStartBlock,
    /// The end block is not followed by `<CR>`.
    BadEnd,
    /// A `<VT>` inside a frame.
    StrayStart,
    /// More than the limit without an end.
    TooLarge,
}

/// Wrap a message in an MLLP frame.
#[must_use]
pub fn frame(payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 3);
    out.push(VT);
    out.extend_from_slice(payload);
    out.extend_from_slice(&[FS, CR]);
    out
}

/// Remove and return the first complete frame's payload from the front of `buf`, `Ok(None)` when
/// more bytes are needed. Line breaks between frames are tolerated (some senders add them); any
/// other byte before `<VT>` is an error.
///
/// # Errors
///
/// [`FrameError`] when the stream is not valid MLLP; the caller closes the connection.
pub fn take_frame(buf: &mut Vec<u8>, max: usize) -> Result<Option<Vec<u8>>, FrameError> {
    let skip = buf
        .iter()
        .take_while(|b| matches!(**b, b'\r' | b'\n'))
        .count();
    if skip > 0 {
        buf.drain(..skip);
    }
    let Some(&first) = buf.first() else {
        return Ok(None);
    };
    if first != VT {
        return Err(FrameError::NoStartBlock);
    }
    let body = &buf[1..];
    match body.iter().position(|b| *b == FS) {
        None => {
            if body.contains(&VT) {
                return Err(FrameError::StrayStart);
            }
            if body.len() > max {
                return Err(FrameError::TooLarge);
            }
            Ok(None)
        }
        Some(end) => {
            if end > max {
                return Err(FrameError::TooLarge);
            }
            if body[..end].contains(&VT) {
                return Err(FrameError::StrayStart);
            }
            match body.get(end + 1) {
                None => Ok(None), // the CR has not arrived yet
                Some(&CR) => {
                    let payload = body[..end].to_vec();
                    buf.drain(..end + 3);
                    Ok(Some(payload))
                }
                Some(_) => Err(FrameError::BadEnd),
            }
        }
    }
}

/// `YYYYMMDDHHMMSS` (UTC) for a unix time, as HL7 timestamps are written.
#[must_use]
pub fn hl7_timestamp(unix: i64) -> String {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}{:02}{:02}{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// The listener's settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// Where to listen.
    pub bind: SocketAddr,
    /// The audit agent for every message.
    pub agent: String,
    /// How many connections are served at once.
    pub max_connections: usize,
    /// How long a connection may be silent.
    pub idle: Duration,
    /// The largest message.
    pub max_frame: usize,
}

impl Config {
    /// A listener for tests and local use: loopback, the default limits.
    #[must_use]
    pub fn local(bind: SocketAddr) -> Config {
        Config {
            bind,
            agent: "mllp:adt-feed".into(),
            max_connections: 8,
            idle: Duration::from_secs(60),
            max_frame: MAX_FRAME,
        }
    }

    /// Settings from the environment, or `None` when `EPR_MLLP_BIND` is not set (the default).
    ///
    /// # Errors
    ///
    /// A message when a variable is malformed, or when the address is not loopback and
    /// `EPR_MLLP_ALLOW_REMOTE` is not `true`: an unauthenticated, unencrypted port is never opened
    /// beyond the machine by accident.
    pub fn from_env() -> Result<Option<Config>, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// [`Config::from_env`] over any lookup, so the rules are testable.
    ///
    /// # Errors
    ///
    /// As [`Config::from_env`].
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Option<Config>, String> {
        let set = |k: &str| {
            get(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let Some(bind) = set("EPR_MLLP_BIND") else {
            return Ok(None);
        };
        let bind: SocketAddr = bind
            .parse()
            .map_err(|_| format!("EPR_MLLP_BIND {bind:?} is not an address like 127.0.0.1:2575"))?;
        let allow_remote =
            set("EPR_MLLP_ALLOW_REMOTE").is_some_and(|v| v.eq_ignore_ascii_case("true"));
        if !is_loopback(bind.ip()) && !allow_remote {
            return Err(format!(
                "EPR_MLLP_BIND {bind} is not a loopback address. MLLP has no authentication or encryption: \
                 set EPR_MLLP_ALLOW_REMOTE=true only behind a TLS terminator or a private network"
            ));
        }
        let number = |k: &str, default: u64, lo: u64, hi: u64| -> Result<u64, String> {
            match set(k) {
                None => Ok(default),
                Some(v) => v
                    .parse::<u64>()
                    .ok()
                    .filter(|n| (lo..=hi).contains(n))
                    .ok_or_else(|| format!("{k} must be a whole number from {lo} to {hi}")),
            }
        };
        let agent = set("EPR_MLLP_AGENT").unwrap_or_else(|| "mllp:adt-feed".into());
        if agent.len() > 100 || agent.chars().any(char::is_control) {
            return Err(
                "EPR_MLLP_AGENT must be up to 100 characters, without control characters".into(),
            );
        }
        Ok(Some(Config {
            bind,
            agent,
            max_connections: usize::try_from(number("EPR_MLLP_MAX_CONNECTIONS", 8, 1, 256)?)
                .unwrap_or(8),
            idle: Duration::from_secs(number("EPR_MLLP_IDLE_SECONDS", 60, 1, 3600)?),
            max_frame: MAX_FRAME,
        }))
    }
}

fn is_loopback(ip: IpAddr) -> bool {
    ip.is_loopback()
}

static ACK_COUNTER: AtomicU64 = AtomicU64::new(1);

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Serve connections until the task is dropped. Each connection runs in its own task.
pub async fn serve(listener: TcpListener, db: DatabaseConnection, config: Config) {
    let slots = Arc::new(Semaphore::new(config.max_connections));
    let config = Arc::new(config);
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "MLLP accept failed");
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };
        let Ok(permit) = Arc::clone(&slots).try_acquire_owned() else {
            tracing::warn!(%peer, "MLLP connection refused: too many connections");
            drop(stream);
            continue;
        };
        let (db, config) = (db.clone(), Arc::clone(&config));
        tokio::spawn(async move {
            connection(stream, peer, &db, &config).await;
            drop(permit);
        });
    }
}

async fn connection(
    mut stream: TcpStream,
    peer: SocketAddr,
    db: &DatabaseConnection,
    config: &Config,
) {
    let who = FeedIdentity {
        agent: config.agent.clone(),
        peer: Some(peer.ip().to_string()),
    };
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        // Answer every complete frame already buffered before reading more.
        loop {
            let payload = match take_frame(&mut buf, config.max_frame) {
                Ok(Some(p)) => p,
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!(%peer, error = ?e, "MLLP stream is not valid; closing");
                    return;
                }
            };
            let Ok(text) = String::from_utf8(payload) else {
                tracing::warn!(%peer, "MLLP message is not UTF-8; closing");
                return;
            };
            let id = format!("EPR{}", ACK_COUNTER.fetch_add(1, Ordering::Relaxed));
            match adt_feed::process(db, &text, &who, &id, &hl7_timestamp(now())).await {
                Some(ack) => {
                    if stream.write_all(&frame(ack.as_bytes())).await.is_err() {
                        return;
                    }
                }
                None => {
                    tracing::warn!(%peer, "MLLP frame is not an HL7 message; closing");
                    return;
                }
            }
        }
        match tokio::time::timeout(config.idle, stream.read(&mut chunk)).await {
            Ok(Ok(0)) | Err(_) | Ok(Err(_)) => return,
            Ok(Ok(n)) => buf.extend_from_slice(&chunk[..n]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(buf: &mut Vec<u8>) -> Result<Option<Vec<u8>>, FrameError> {
        take_frame(buf, 100)
    }

    #[test]
    fn a_frame_round_trips() {
        let mut buf = frame(b"MSH|^~\\&|A");
        assert_eq!(take(&mut buf), Ok(Some(b"MSH|^~\\&|A".to_vec())));
        assert!(buf.is_empty());
    }

    #[test]
    fn two_frames_in_one_read_and_one_frame_over_many_reads() {
        let mut buf = [frame(b"one"), frame(b"two")].concat();
        assert_eq!(take(&mut buf), Ok(Some(b"one".to_vec())));
        assert_eq!(take(&mut buf), Ok(Some(b"two".to_vec())));
        assert_eq!(take(&mut buf), Ok(None));
        // Byte by byte: nothing is returned until the CR arrives.
        let whole = frame(b"hello");
        let mut buf = Vec::new();
        for (i, b) in whole.iter().enumerate() {
            buf.push(*b);
            let got = take(&mut buf).unwrap();
            if i + 1 < whole.len() {
                assert_eq!(got, None, "byte {i}");
            } else {
                assert_eq!(got, Some(b"hello".to_vec()));
            }
        }
    }

    #[test]
    fn line_breaks_between_frames_are_tolerated_and_other_bytes_are_not() {
        let mut buf = [b"\r\n".to_vec(), frame(b"a"), b"\n".to_vec(), frame(b"b")].concat();
        assert_eq!(take(&mut buf), Ok(Some(b"a".to_vec())));
        assert_eq!(take(&mut buf), Ok(Some(b"b".to_vec())));
        let mut junk = b"GET / HTTP/1.1\r\n".to_vec();
        assert_eq!(take(&mut junk), Err(FrameError::NoStartBlock));
        let mut junk = [b"x".to_vec(), frame(b"a")].concat();
        assert_eq!(take(&mut junk), Err(FrameError::NoStartBlock));
    }

    #[test]
    fn a_bad_end_a_stray_start_and_too_much_data_are_errors() {
        let mut bad_end = vec![VT, b'a', FS, b'x'];
        assert_eq!(take(&mut bad_end), Err(FrameError::BadEnd));
        let mut stray = vec![VT, b'a', VT, b'b', FS, CR];
        assert_eq!(take(&mut stray), Err(FrameError::StrayStart));
        let mut stray_open = vec![VT, b'a', VT, b'b'];
        assert_eq!(take(&mut stray_open), Err(FrameError::StrayStart));
        let mut big = vec![VT];
        big.extend(std::iter::repeat_n(b'a', 101));
        assert_eq!(take(&mut big), Err(FrameError::TooLarge));
        let mut big_done = vec![VT];
        big_done.extend(std::iter::repeat_n(b'a', 101));
        big_done.extend([FS, CR]);
        assert_eq!(take(&mut big_done), Err(FrameError::TooLarge));
        // Exactly at the limit is fine.
        let mut ok = frame(&[b'a'; 100]);
        assert!(take(&mut ok).unwrap().is_some());
    }

    #[test]
    fn an_empty_frame_is_a_frame_with_no_message() {
        let mut buf = frame(b"");
        assert_eq!(take(&mut buf), Ok(Some(Vec::new())));
    }

    #[test]
    fn hl7_timestamps_are_utc_and_exact() {
        assert_eq!(hl7_timestamp(0), "19700101000000");
        assert_eq!(hl7_timestamp(951_782_400), "20000229000000"); // a leap day
        assert_eq!(hl7_timestamp(1_792_573_212), "20261021090012");
        assert_eq!(hl7_timestamp(86_399), "19700101235959");
    }

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: std::collections::HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn the_listener_is_off_unless_asked_for_and_never_open_to_the_world_by_accident() {
        assert!(Config::from_lookup(env(&[])).unwrap().is_none());
        assert!(
            Config::from_lookup(env(&[("EPR_MLLP_BIND", "  ")]))
                .unwrap()
                .is_none()
        );
        let c = Config::from_lookup(env(&[("EPR_MLLP_BIND", "127.0.0.1:2575")]))
            .unwrap()
            .unwrap();
        assert_eq!(
            (c.agent.as_str(), c.max_connections, c.idle),
            ("mllp:adt-feed", 8, Duration::from_secs(60))
        );
        assert!(
            Config::from_lookup(env(&[("EPR_MLLP_BIND", "[::1]:2575")]))
                .unwrap()
                .is_some()
        );
        for open in ["0.0.0.0:2575", "10.1.2.3:2575", "[::]:2575"] {
            let e = Config::from_lookup(env(&[("EPR_MLLP_BIND", open)])).unwrap_err();
            assert!(e.contains("EPR_MLLP_ALLOW_REMOTE"), "{e}");
            assert!(
                Config::from_lookup(env(&[
                    ("EPR_MLLP_BIND", open),
                    ("EPR_MLLP_ALLOW_REMOTE", "true")
                ]))
                .unwrap()
                .is_some(),
                "{open}"
            );
        }
        assert!(
            Config::from_lookup(env(&[
                ("EPR_MLLP_BIND", "0.0.0.0:2575"),
                ("EPR_MLLP_ALLOW_REMOTE", "yes")
            ]))
            .is_err()
        );
    }

    #[test]
    fn malformed_settings_are_refused_not_defaulted() {
        let bind = ("EPR_MLLP_BIND", "127.0.0.1:2575");
        assert!(Config::from_lookup(env(&[("EPR_MLLP_BIND", "localhost")])).is_err());
        assert!(Config::from_lookup(env(&[bind, ("EPR_MLLP_MAX_CONNECTIONS", "0")])).is_err());
        assert!(Config::from_lookup(env(&[bind, ("EPR_MLLP_MAX_CONNECTIONS", "999")])).is_err());
        assert!(Config::from_lookup(env(&[bind, ("EPR_MLLP_IDLE_SECONDS", "soon")])).is_err());
        assert!(Config::from_lookup(env(&[bind, ("EPR_MLLP_AGENT", "a\nb")])).is_err());
        let c = Config::from_lookup(env(&[
            bind,
            ("EPR_MLLP_AGENT", "mllp:ward-system"),
            ("EPR_MLLP_IDLE_SECONDS", "5"),
            ("EPR_MLLP_MAX_CONNECTIONS", "2"),
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(
            (c.agent.as_str(), c.idle, c.max_connections),
            ("mllp:ward-system", Duration::from_secs(5), 2)
        );
    }
}

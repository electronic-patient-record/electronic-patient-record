//! An MLLP listener for HL7 v2 ADT feeds (task OR-3).
//!
//! MLLP is a thin frame around an HL7 message on a TCP stream: `<VT> message <FS><CR>` (bytes
//! `0x0B`, then `0x1C 0x0D`). The listener reads frames, hands each message to
//! [`crate::adt_feed::process`] and writes the acknowledgement back in a frame.
//!
//! **MLLP has no authentication and no encryption.** The listener is therefore off unless
//! `EPR_MLLP_BIND` is set, refuses a non-loopback address unless `EPR_MLLP_ALLOW_REMOTE=true`, and
//! when it is allowed to listen beyond the machine it must use TLS, sit behind a TLS terminator, or be
//! on a private network that the deployer controls.
//!
//! **TLS (IH-10)**, rustls with ring only: `EPR_MLLP_TLS_CERT_FILE` and `EPR_MLLP_TLS_KEY_FILE` (PEM, both or neither)
//! turn it on, and then the port speaks only TLS: a client that does not complete a handshake is closed, and nothing is
//! ever served in plain text. `EPR_MLLP_TLS_CLIENT_CA_FILE` (PEM) also requires every sender to present a certificate
//! that chains to it, which is the only way MLLP here can know who is sending, and `EPR_MLLP_TLS_CLIENT_NAMES` (a comma
//! list of DNS names or addresses, which needs the client CA) also requires that certificate to be valid for one of
//! them: the name it matched is recorded on every audit row as `peer_certificate`, and a certificate valid for none of
//! them closes the connection before anything is read. A file that is missing, empty or
//! unreadable, holds no certificate or no key, or a key that does not fit the certificate, stops the service starting. Whatever the sender says about itself in `MSH` is not used to
//! decide anything: the audit agent is `EPR_MLLP_AGENT`, and the peer address is recorded.
//!
//! Limits: one frame is at most 1 MiB; a connection that is silent for `EPR_MLLP_IDLE_SECONDS` (60)
//! is closed; at most `EPR_MLLP_MAX_CONNECTIONS` (8) are served at once and the rest are closed on
//! accept. A byte stream that does not start a frame with `<VT>`, a frame that never ends, a stray
//! `<VT>` inside a frame and text that is not UTF-8 each close the connection: nothing is guessed.

use crate::adt_feed::{self, FeedIdentity, KeptAnswer, SenderRules};
use sqlx::PgPool;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
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
    let Some(body) = buf.get(1..) else {
        return Ok(None); // not reachable: the first byte was read above
    };
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
            if body.get(..end).is_some_and(|head| head.contains(&VT)) {
                return Err(FrameError::StrayStart);
            }
            match body.get(end + 1) {
                None => Ok(None), // the CR has not arrived yet
                Some(&CR) => {
                    let Some(payload) = body.get(..end).map(<[u8]>::to_vec) else {
                        return Err(FrameError::BadEnd); // not reachable: `end` is a position in `body`
                    };
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
    /// How a refusal kept in the work queue is answered (`EPR_MLLP_KEPT_ANSWER`, `AR` by default).
    pub kept_answer: KeptAnswer,
    /// TLS, when configured: then the port speaks nothing else (IH-10).
    pub tls: Option<Arc<rustls::ServerConfig>>,
    /// The names a client certificate must be valid for, one of them (`EPR_MLLP_TLS_CLIENT_NAMES`); empty checks none.
    pub client_names: Vec<String>,
    /// The processing id this deployment takes (`EPR_MLLP_PROCESSING_ID`: `P`, `T` or `D`), or any when unset (IH-2).
    pub processing_id: Option<char>,
    /// What the sender may send (`EPR_MLLP_SENDING_APPLICATIONS`, `EPR_MLLP_VERSIONS`, `EPR_MLLP_EVENTS`; IH-6).
    pub rules: SenderRules,
}

/// A TLS server configuration from PEM files, with ring's provider chosen explicitly (it is the only one
/// in the tree). With `client_ca`, every client must present a certificate that chains to it.
///
/// # Errors
///
/// A message naming the variable when a file is missing, empty or unreadable, holds no certificate or no private key,
/// or the key does not fit the certificate.
pub fn tls_config(
    cert_file: &str,
    key_file: &str,
    client_ca_file: Option<&str>,
) -> Result<Arc<rustls::ServerConfig>, String> {
    use rustls_pki_types::pem::PemObject;
    use rustls_pki_types::{CertificateDer, PrivateKeyDer};
    let read = |var: &str, path: &str| -> Result<Vec<u8>, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("{var} {path:?} cannot be read: {e}"))?;
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Err(format!("{var} {path:?} is empty"));
        }
        Ok(bytes)
    };
    let certs = |var: &str, path: &str| -> Result<Vec<CertificateDer<'static>>, String> {
        let bytes = read(var, path)?;
        let certs = CertificateDer::pem_slice_iter(&bytes)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("{var} {path:?} is not PEM certificates: {e}"))?;
        if certs.is_empty() {
            return Err(format!("{var} {path:?} holds no certificate"));
        }
        Ok(certs)
    };
    let chain = certs("EPR_MLLP_TLS_CERT_FILE", cert_file)?;
    let key =
        PrivateKeyDer::from_pem_slice(&read("EPR_MLLP_TLS_KEY_FILE", key_file)?).map_err(|e| {
            format!("EPR_MLLP_TLS_KEY_FILE {key_file:?} holds no usable private key: {e}")
        })?;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ServerConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("MLLP TLS: {e}"))?;
    let builder = match client_ca_file {
        None => builder.with_no_client_auth(),
        Some(path) => {
            let mut roots = rustls::RootCertStore::empty();
            for ca in certs("EPR_MLLP_TLS_CLIENT_CA_FILE", path)? {
                roots
                    .add(ca)
                    .map_err(|e| format!("EPR_MLLP_TLS_CLIENT_CA_FILE {path:?}: {e}"))?;
            }
            let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
                Arc::new(roots),
                provider,
            )
            .build()
            .map_err(|e| format!("EPR_MLLP_TLS_CLIENT_CA_FILE {path:?}: {e}"))?;
            builder.with_client_cert_verifier(verifier)
        }
    };
    builder
        .with_single_cert(chain, key)
        .map(Arc::new)
        .map_err(|e| format!("EPR_MLLP_TLS_KEY_FILE {key_file:?} does not fit EPR_MLLP_TLS_CERT_FILE {cert_file:?}: {e}"))
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
            kept_answer: KeptAnswer::Reject,
            tls: None,
            client_names: Vec::new(),
            processing_id: None,
            rules: SenderRules::default(),
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
        Self::from_lookup(crate::runtime::setting)
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
        let kept_answer = match set("EPR_MLLP_KEPT_ANSWER").as_deref() {
            None | Some("AR") => KeptAnswer::Reject,
            Some("AE") => KeptAnswer::Error,
            Some(other) => {
                return Err(format!(
                    "EPR_MLLP_KEPT_ANSWER {other:?} must be AR (the default: kept for a person, do not resend) or AE"
                ));
            }
        };
        let tls = match (set("EPR_MLLP_TLS_CERT_FILE"), set("EPR_MLLP_TLS_KEY_FILE")) {
            (None, None) => {
                if set("EPR_MLLP_TLS_CLIENT_CA_FILE").is_some() {
                    return Err("EPR_MLLP_TLS_CLIENT_CA_FILE needs EPR_MLLP_TLS_CERT_FILE and EPR_MLLP_TLS_KEY_FILE".into());
                }
                None
            }
            (Some(cert), Some(key)) => Some(tls_config(
                &cert,
                &key,
                set("EPR_MLLP_TLS_CLIENT_CA_FILE").as_deref(),
            )?),
            _ => {
                return Err(
                    "set both EPR_MLLP_TLS_CERT_FILE and EPR_MLLP_TLS_KEY_FILE, or neither".into(),
                );
            }
        };
        let client_names: Vec<String> =
            set("EPR_MLLP_TLS_CLIENT_NAMES").map_or_else(Vec::new, |v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(String::from)
                    .collect()
            });
        if !client_names.is_empty() && set("EPR_MLLP_TLS_CLIENT_CA_FILE").is_none() {
            return Err("EPR_MLLP_TLS_CLIENT_NAMES needs EPR_MLLP_TLS_CLIENT_CA_FILE".into());
        }
        if let Some(bad) = client_names
            .iter()
            .find(|n| rustls_pki_types::ServerName::try_from(n.as_str()).is_err())
        {
            return Err(format!(
                "EPR_MLLP_TLS_CLIENT_NAMES entry {bad:?} must be a DNS name or an IP address"
            ));
        }
        let processing_id = match set("EPR_MLLP_PROCESSING_ID").as_deref() {
            None => None,
            Some("P") => Some('P'),
            Some("T") => Some('T'),
            Some("D") => Some('D'),
            Some(other) => {
                return Err(format!(
                    "EPR_MLLP_PROCESSING_ID {other:?} must be P (production), T (training) or D (debugging)"
                ));
            }
        };
        let list = |k: &str| -> Vec<String> {
            set(k).map_or_else(Vec::new, |v| {
                v.split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect()
            })
        };
        let rules = SenderRules {
            applications: list("EPR_MLLP_SENDING_APPLICATIONS"),
            versions: list("EPR_MLLP_VERSIONS"),
            events: list("EPR_MLLP_EVENTS"),
        };
        let clean = |x: &str| {
            x.chars()
                .all(|c| c.is_ascii_alphanumeric() || "^._- ".contains(c))
                && x.len() <= 180
        };
        if let Some(bad) = rules
            .applications
            .iter()
            .find(|a| !clean(a) || a.matches('^').count() > 1)
        {
            return Err(format!(
                "EPR_MLLP_SENDING_APPLICATIONS entry {bad:?} must be APP or APP^FACILITY in letters, digits, '.', '_', '-' or spaces"
            ));
        }
        if let Some(bad) = rules
            .versions
            .iter()
            .find(|v| v.is_empty() || !v.chars().all(|c| c.is_ascii_digit() || c == '.'))
        {
            return Err(format!(
                "EPR_MLLP_VERSIONS entry {bad:?} must be a version such as 2.5 or 2.5.1"
            ));
        }
        if let Some(bad) = rules
            .events
            .iter()
            .find(|e| !crate::interop::hl7_adt::HANDLED_EVENTS.contains(&e.as_str()))
        {
            return Err(format!(
                "EPR_MLLP_EVENTS entry {bad:?} is not an event this feed handles ({})",
                crate::interop::hl7_adt::HANDLED_EVENTS.join(", ")
            ));
        }
        Ok(Some(Config {
            bind,
            agent,
            kept_answer,
            tls,
            client_names,
            processing_id,
            rules,
            max_connections: usize::try_from(number("EPR_MLLP_MAX_CONNECTIONS", 8, 1, 256)?)
                .unwrap_or(8),
            idle: Duration::from_secs(number("EPR_MLLP_IDLE_SECONDS", 60, 1, 3600)?),
            max_frame: MAX_FRAME,
        }))
    }
}

/// Whether an address is a loopback one. A bind to any other address is refused unless `EPR_MLLP_ALLOW_REMOTE=true`,
/// because MLLP has no authentication or encryption.
fn is_loopback(ip: IpAddr) -> bool {
    ip.is_loopback()
}

static ACK_COUNTER: AtomicU64 = AtomicU64::new(1);

/// The current time in whole seconds since the Unix epoch; a clock before 1970 reads as 0 and one beyond range saturates.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Serve connections until the task is dropped. Each connection runs in its own task.
pub async fn serve(listener: TcpListener, db: PgPool, config: Config) {
    let (_keep, stop) = tokio::sync::watch::channel(false);
    serve_until(listener, db, config, stop).await;
}

/// How long a stopping listener waits for the messages it is already handling (IH-7).
pub const STOP_WAIT: Duration = Duration::from_secs(10);

/// Serve connections until `stop` turns true, then drain (IH-7): stop accepting, close idle connections, let each
/// message already being handled finish (its change committed and its answer sent), and wait up to [`STOP_WAIT`] for
/// them. A connection still working after that is aborted: its transaction rolls back and it never answers `AA`, so the
/// sender sends it again. Frames already read but not yet started are left unanswered, for the same reason.
pub async fn serve_until(
    listener: TcpListener,
    db: PgPool,
    config: Config,
    mut stop: tokio::sync::watch::Receiver<bool>,
) {
    let slots = Arc::new(Semaphore::new(config.max_connections));
    let config = Arc::new(config);
    let mut connections = tokio::task::JoinSet::new();
    loop {
        let accepted = tokio::select! {
            a = listener.accept() => a,
            _ = stop.changed() => break,
        };
        // Reap finished connections so the set does not grow.
        while connections.try_join_next().is_some() {}
        let (stream, peer) = match accepted {
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
        let (db, config, stop) = (db.clone(), Arc::clone(&config), stop.clone());
        connections.spawn(async move {
            match &config.tls {
                None => connection(stream, peer, None, &db, &config, stop).await,
                // TLS only: a handshake that fails or does not finish within the idle time closes the connection,
                // and nothing is read in plain text (IH-10).
                Some(tls) => {
                    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::clone(tls));
                    match tokio::time::timeout(config.idle, acceptor.accept(stream)).await {
                        Ok(Ok(secure)) if config.client_names.is_empty() => {
                            connection(secure, peer, None, &db, &config, stop).await;
                        }
                        Ok(Ok(secure)) => {
                            let named = secure
                                .get_ref()
                                .1
                                .peer_certificates()
                                .and_then(<[_]>::first)
                                .and_then(|c| certificate_name(c, &config.client_names));
                            match named {
                                Some(name) => {
                                    connection(secure, peer, Some(name), &db, &config, stop).await;
                                }
                                None => tracing::warn!(
                                    %peer,
                                    "MLLP client certificate is valid for none of EPR_MLLP_TLS_CLIENT_NAMES; closing"
                                ),
                            }
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(%peer, error = %e, "MLLP TLS handshake failed; closing")
                        }
                        Err(_) => tracing::warn!(%peer, "MLLP TLS handshake timed out; closing"),
                    }
                }
            }
            drop(permit);
        });
    }
    drop(listener);
    let open = connections.len();
    let drained = tokio::time::timeout(STOP_WAIT, async {
        while connections.join_next().await.is_some() {}
    })
    .await
    .is_ok();
    if drained {
        tracing::info!(
            open,
            "MLLP listener stopped; every message in hand was finished"
        );
    } else {
        tracing::error!(
            left = connections.len(),
            seconds = STOP_WAIT.as_secs(),
            "MLLP listener stopped with messages unfinished; they were rolled back and are not acknowledged"
        );
        connections.abort_all();
    }
}

/// The first of `names` that a client's end-entity certificate is valid for (its subject alternative names, as webpki
/// checks a server's), or `None` when it is valid for none or cannot be read. The chain was already verified by the
/// handshake; this only names the sender.
#[must_use]
pub fn certificate_name(
    cert: &rustls_pki_types::CertificateDer<'_>,
    names: &[String],
) -> Option<String> {
    let cert = webpki::EndEntityCert::try_from(cert).ok()?;
    names
        .iter()
        .find(|n| {
            rustls_pki_types::ServerName::try_from(n.as_str())
                .is_ok_and(|name| cert.verify_is_valid_for_subject_name(&name).is_ok())
        })
        .cloned()
}

/// A listener the service started, and how to stop it.
pub struct Running {
    stop: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl Running {
    /// Start [`serve_until`].
    #[must_use]
    pub fn start(listener: TcpListener, db: PgPool, config: Config) -> Self {
        let (stop, stopped) = tokio::sync::watch::channel(false);
        adt_feed::set_configured(FeedIdentity {
            agent: config.agent.clone(),
            peer: None,
            peer_certificate: None,
            kept_answer: config.kept_answer,
            processing_id: config.processing_id,
            rules: config.rules.clone(),
        });
        let task = tokio::spawn(serve_until(listener, db, config, stopped));
        Running { stop, task }
    }

    /// Drain and stop: answers whether the listener ended within [`STOP_WAIT`] and a second.
    pub async fn stop(self) -> bool {
        let _ = self.stop.send(true);
        tokio::time::timeout(STOP_WAIT + Duration::from_secs(1), self.task)
            .await
            .is_ok_and(|r| r.is_ok())
    }
}

/// The listener the service started, kept for [`stop_kept`].
static KEPT: std::sync::Mutex<Option<Running>> = std::sync::Mutex::new(None);

/// Keep `running` until [`stop_kept`].
pub fn keep(running: Running) {
    *KEPT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(running);
}

/// Drain and stop the kept listener, if any. Answers whether there was none or it stopped in time.
pub async fn stop_kept() -> bool {
    let kept = KEPT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    match kept {
        Some(running) => running.stop().await,
        None => true,
    }
}

/// Serve one connection: read bytes, answer every complete frame in the buffer with [`adt_feed::process`] (an `AA`, `AE` or
/// `AR` acknowledgement, framed), and close on a stream that is not valid MLLP, a message that is not UTF-8 or a frame
/// that is not an HL7 message. Each message is given an acknowledgement id from a process-wide counter.
async fn connection<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    peer: SocketAddr,
    peer_certificate: Option<String>,
    db: &PgPool,
    config: &Config,
    mut stop: tokio::sync::watch::Receiver<bool>,
) {
    let who = FeedIdentity {
        agent: config.agent.clone(),
        peer: Some(peer.ip().to_string()),
        peer_certificate,
        kept_answer: config.kept_answer,
        processing_id: config.processing_id,
        rules: config.rules.clone(),
    };
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        // Answer every complete frame already buffered before reading more; when stopping, start no new one.
        loop {
            if *stop.borrow() {
                return;
            }
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
                    // Its shape only, cut short: every value is masked (IH-4).
                    let shape: String = crate::interop::redact::er7(&text)
                        .chars()
                        .take(200)
                        .collect();
                    tracing::warn!(%peer, %shape, "MLLP frame is not an HL7 message; closing");
                    return;
                }
            }
        }
        if *stop.borrow() {
            return;
        }
        let read = tokio::select! {
            r = tokio::time::timeout(config.idle, stream.read(&mut chunk)) => r,
            // Stopping: an idle connection closes now (IH-7).
            _ = stop.changed() => return,
        };
        match read {
            Ok(Ok(0)) | Err(_) | Ok(Err(_)) => return,
            Ok(Ok(n)) => match chunk.get(..n) {
                Some(read) => buf.extend_from_slice(read),
                None => return, // not reachable: a read gives at most the chunk
            },
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
        // IH-6: what a sender may send, checked at start.
        let rules = |pairs: &[(&str, &str)]| {
            let mut all = vec![bind];
            all.extend_from_slice(pairs);
            Config::from_lookup(env(&all))
        };
        let ok = rules(&[
            ("EPR_MLLP_SENDING_APPLICATIONS", "REG^CLINIC, PAS"),
            ("EPR_MLLP_VERSIONS", "2.5,2.5.1"),
            ("EPR_MLLP_EVENTS", "A08, A28"),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(ok.rules.applications, ["REG^CLINIC", "PAS"]);
        assert_eq!(ok.rules.versions, ["2.5", "2.5.1"]);
        assert_eq!(ok.rules.events, ["A08", "A28"]);
        assert_eq!(rules(&[]).unwrap().unwrap().rules, SenderRules::default());
        for bad in [
            ("EPR_MLLP_SENDING_APPLICATIONS", "A^B^C"),
            ("EPR_MLLP_SENDING_APPLICATIONS", "A|B"),
            ("EPR_MLLP_VERSIONS", "two"),
            ("EPR_MLLP_EVENTS", "R01"),
        ] {
            assert!(rules(&[bad]).is_err(), "{bad:?}");
        }
        // IH-1: how a kept refusal is answered; AR unless set to AE; anything else stops the start.
        let kept = |v: &str| Config::from_lookup(env(&[bind, ("EPR_MLLP_KEPT_ANSWER", v)]));
        assert_eq!(kept("").unwrap().unwrap().kept_answer, KeptAnswer::Reject);
        assert_eq!(kept("AR").unwrap().unwrap().kept_answer, KeptAnswer::Reject);
        assert_eq!(
            kept(" AE ").unwrap().unwrap().kept_answer,
            KeptAnswer::Error
        );
        assert!(kept("ae").is_err());
        assert!(kept("AA").is_err());
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

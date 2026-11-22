//! HL7 v2 patient-administration feeds (ADT) and their acknowledgements (crates `er7`, `hl7`).
//!
//! Real ADT feeds are rarely conformant, so the patient is read **leniently** with `er7` (a
//! missing optional segment or a local `Z` segment does not stop a registration), and the
//! dictionary check from `hl7::v2` is reported as `warnings` rather than as a refusal. What *does*
//! refuse a message is the identity rules the rest of the service already applies
//! (`patients_store::validate`): a patient with no family name or a bad national health number is not
//! registered because a feed said so.
//!
//! The reply is an `ACK` built by `hl7::v2`: `AA` accepted, `AE` the message was understood but
//! its patient is unacceptable, `AR` the message type is not one this adapter handles.
//!
//! Not here: a network listener (MLLP, OR-3), storing the patient, matching it to an existing one,
//! or any segment other than `MSH`, `EVN` and `PID`.

use crate::death::Occurred;
use crate::patients_store;
use electronic_patient_record_matcher::{
    Address, AdministrativeSex, HumanName, Identifier, NATIONAL_NUMBER_SYSTEM, Patient, Telecom,
};

/// ADT events that register or update a patient: admit, register, update, add person.
pub const HANDLED_EVENTS: &[&str] = &["A01", "A04", "A08", "A28", "A31"];

/// A patient read from an ADT message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inbound {
    /// The trigger event, such as `A01`.
    pub event: String,
    /// MSH-10, which the acknowledgement echoes.
    pub control_id: String,
    /// The identity record.
    pub patient: Patient,
    /// The message reports a death (PID-29, PID-30), and when. This is a report, never a fact.
    pub death: Option<Occurred>,
    /// Things worth knowing that did not stop the message: dictionary findings, identifiers that
    /// were skipped.
    pub warnings: Vec<String>,
    /// When the sender says the event happened (EVN-2), or else when it built the message (MSH-7), read by
    /// [`message_time`]; `None` when neither is a usable time. The feed uses it so that a late message never undoes
    /// a newer one (IH-5).
    pub message_time: Option<jiff::Timestamp>,
}

/// An HL7 v2 `DTM` to at least the minute (`YYYYMMDDHHMM[SS[.S…]][+/-ZZZZ]`) as a UTC time. A time with no offset is
/// read as UTC: one sender's times compare with each other, which is all the feed asks of them. Anything shorter
/// than a minute, or not a real time, is `None`.
#[must_use]
pub fn message_time(ts: &str) -> Option<jiff::Timestamp> {
    let ts = ts.trim();
    let (main, offset) = match ts.find(['+', '-']) {
        Some(i) => (&ts[..i], Some(&ts[i..])),
        None => (ts, None),
    };
    let main = main.split('.').next()?;
    if !main.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let full = match main.len() {
        12 => format!("{main}00"),
        14 => main.to_string(),
        _ => return None,
    };
    let naive = jiff::civil::DateTime::strptime("%Y%m%d%H%M%S", &full).ok()?;
    let seconds = match offset {
        None => 0,
        Some(o) => {
            let (sign, digits) = o.split_at(1);
            if digits.len() != 4 || !digits.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let hours: i32 = digits[..2].parse().ok()?;
            let minutes: i32 = digits[2..].parse().ok()?;
            if hours > 14 || minutes > 59 {
                return None;
            }
            let s = hours * 3600 + minutes * 60;
            if sign == "-" { -s } else { s }
        }
    };
    jiff::tz::Offset::from_seconds(seconds)
        .ok()?
        .to_timestamp(naive)
        .ok()
}

/// The reason for text that is not an HL7 v2 message: fixed, because the parser's own error quotes the text (IH-4).
pub const NOT_HL7: &str = "not an HL7 v2 message: it does not start with a readable MSH segment";

/// A finding about a message with every quoted value taken out (IH-4): the path and the reason stay, the value goes.
/// The HL7 dictionary's findings quote the field they are about (`PID[1]-7[1].1: "…" is not a valid DTM value`), and a
/// field holds patient data, so no finding leaves the reader with one. An unclosed quote withholds the rest.
#[must_use]
pub fn withhold_values(finding: &str) -> String {
    let mut out = String::with_capacity(finding.len());
    let mut quoted = false;
    for c in finding.chars() {
        if c == '"' {
            if !quoted {
                out.push_str("(value withheld)");
            }
            quoted = !quoted;
        } else if !quoted {
            out.push(c);
        }
    }
    out
}

/// Why a message was not accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// Not an HL7 v2 message at all (no usable `MSH`). There is nothing to acknowledge.
    NotHl7(String),
    /// A message type this adapter does not handle. Acknowledged `AR`.
    Unsupported(String),
    /// A handled message whose patient is unacceptable. Acknowledged `AE`.
    Invalid(Vec<String>),
    /// Refused and kept for a person, so the sender should not send it again (IH-1): acknowledged `AR`, the reasons
    /// in `MSA-3`. The reader never returns this; the feed does, once the message is stored.
    Kept(Vec<String>),
}

/// The administrative sex an HL7 code names: `M`, `F`, `O`; and `U`, `A` or `N` (unknown, ambiguous, not applicable) all
/// become unknown, because none is a usable answer. Any other code is `None`.
fn sex(code: &str) -> Option<AdministrativeSex> {
    match code.trim().to_ascii_uppercase().as_str() {
        "M" => Some(AdministrativeSex::Male),
        "F" => Some(AdministrativeSex::Female),
        "O" => Some(AdministrativeSex::Other),
        // Unknown, ambiguous and not-applicable all say "no usable answer".
        "U" | "A" | "N" => Some(AdministrativeSex::Unknown),
        _ => None,
    }
}

/// `YYYY`, `YYYYMM` or `YYYYMMDD` (anything after, such as a time, is ignored) as ISO 8601.
fn birth_date(ts: &str) -> Option<String> {
    let digits: String = ts.chars().take_while(char::is_ascii_digit).collect();
    match digits.len() {
        4 => Some(digits),
        6 => Some(format!("{}-{}", &digits[..4], &digits[4..6])),
        n if n >= 8 => Some(format!(
            "{}-{}-{}",
            &digits[..4],
            &digits[4..6],
            &digits[6..8]
        )),
        _ => None,
    }
}

/// Why a message that reports a death could not be recorded. Date-free: it reaches the audit log.
pub const DEATH_NOT_APPLIED: &str = "this message reports a death, which was not recorded: nothing was applied, and a person must act on it";

/// Why a report from a feed has no date, when the date it gave could not be used.
pub const UNREADABLE_DEATH_DATE: &str = "the message gave a date that could not be read";

/// Whether PID-29 (date of death) or PID-30 (death indicator) reports a death. Any date counts, even
/// beside an indicator of `N`, as any report does (OQ-BV1). `""` is HL7's explicit null, not a report.
fn reports_a_death(date: &str, indicator: &str) -> bool {
    let given = |s: &str| !s.trim().is_empty() && s.trim() != "\"\"";
    given(date) || indicator.trim().eq_ignore_ascii_case("Y")
}

/// When the death happened, as far as PID-29 says. A date that cannot be read is not a reason to lose
/// the report, so it becomes a date not known. A date that cannot be stored is left to the caller.
fn death_occurred(date: &str) -> Occurred {
    let date = date.trim();
    if date.is_empty() || date == "\"\"" {
        return Occurred::NotKnown {
            reason: "the message gave no date".into(),
        };
    }
    match birth_date(date) {
        Some(at) => Occurred::Known { at },
        None => Occurred::NotKnown {
            reason: UNREADABLE_DEATH_DATE.into(),
        },
    }
}

/// Every repetition of a field, as long as at least one of `probe` components is present.
fn repetitions(message: &er7::Message, field: &str, probes: &[&str]) -> Vec<usize> {
    (1..=20)
        .take_while(|n| {
            probes.iter().any(|p| {
                message
                    .query(&format!("{field}[{n}]{p}"))
                    .ok()
                    .flatten()
                    .is_some_and(|v| !v.is_empty())
            })
        })
        .collect()
}

/// The trimmed text at an HL7 path in a message, or an empty string when the path is absent or unreadable.
fn get(message: &er7::Message, path: &str) -> String {
    message
        .query(path)
        .ok()
        .flatten()
        .map_or_else(String::new, |v| v.trim().to_string())
}

/// Read the patient from an ADT message.
///
/// # Errors
///
/// [`Rejection`], saying which acknowledgement (if any) is owed.
pub fn patient_from_adt(text: &str) -> Result<Inbound, Rejection> {
    // The reader's own error names what it found first, which is the sender's text: only a fixed reason leaves here.
    let message = er7::parse(text).map_err(|_| Rejection::NotHl7(NOT_HL7.to_string()))?;
    let msg_type = get(&message, "MSH-9.1");
    let event = get(&message, "MSH-9.2");
    if msg_type != "ADT" || !HANDLED_EVENTS.contains(&event.as_str()) {
        let mut why = format!(
            "message type {msg_type}^{event} is not handled (ADT^{})",
            HANDLED_EVENTS.join(", ")
        );
        // An A03 discharge for a death lands here: the death is not left unsaid (H-046).
        if reports_a_death(&get(&message, "PID-29"), &get(&message, "PID-30")) {
            why = format!("{why}; {DEATH_NOT_APPLIED}");
        }
        return Err(Rejection::Unsupported(why));
    }
    let mut warnings = Vec::new();
    // The dictionary check is advice here: parse again through the HL7 layer and keep its findings.
    match hl7::v2::parse(text) {
        Ok(m) => warnings.extend(m.validate().iter().map(|f| withhold_values(&f.to_string()))),
        Err(e) => warnings.push(withhold_values(&format!("hl7 v2: {e}"))),
    }
    if message.segment_at("PID", 1).is_none() {
        return Err(Rejection::Invalid(vec!["no PID segment".into()]));
    }

    let names = repetitions(&message, "PID-5", &[".1", ".2"])
        .into_iter()
        .map(|n| HumanName {
            family: Some(get(&message, &format!("PID-5[{n}].1"))).filter(|f| !f.is_empty()),
            given: [".2", ".3"]
                .iter()
                .map(|c| get(&message, &format!("PID-5[{n}]{c}")))
                .filter(|g| !g.is_empty())
                .collect(),
        })
        .collect();

    let mut identifiers = Vec::new();
    for n in repetitions(&message, "PID-3", &[".1"]) {
        let value = get(&message, &format!("PID-3[{n}].1"));
        let authority = get(&message, &format!("PID-3[{n}].4.1"));
        let type_code = get(&message, &format!("PID-3[{n}].5"));
        // `NH` and `NHS` are the type code and assigning authority that senders put on this number:
        // message data, matched as sent.
        if type_code == "NH" || authority.eq_ignore_ascii_case("NHS") {
            identifiers.push(Identifier {
                system: NATIONAL_NUMBER_SYSTEM.into(),
                value: value.chars().filter(|c| !c.is_whitespace()).collect(),
            });
        } else if authority.is_empty() {
            // An identifier with no assigning authority cannot be told apart from another
            // system's; inventing a system for it would be a guess.
            warnings.push(format!(
                "PID-3 repetition {n}: no assigning authority, skipped"
            ));
        } else {
            identifiers.push(Identifier {
                system: format!("urn:hl7v2:{authority}"),
                value,
            });
        }
    }

    let mut telecom = Vec::new();
    for n in repetitions(&message, "PID-13", &[".1", ".4"]) {
        let email = get(&message, &format!("PID-13[{n}].4"));
        let number = get(&message, &format!("PID-13[{n}].1"));
        if !email.is_empty() {
            telecom.push(Telecom {
                system: "email".into(),
                value: email,
            });
        } else if !number.is_empty() {
            telecom.push(Telecom {
                system: "phone".into(),
                value: number,
            });
        }
    }

    let postal = get(&message, "PID-11[1].5");
    let raw_sex = get(&message, "PID-8");
    let raw_birth = get(&message, "PID-7");
    let patient = Patient {
        death_date: None,
        id: None,
        names,
        birth_date: birth_date(&raw_birth),
        sex: sex(&raw_sex),
        identifiers,
        addresses: if postal.is_empty() {
            vec![]
        } else {
            vec![Address {
                postal_code: Some(postal),
            }]
        },
        telecom,
        // Only a person says that an identity is temporary: a message never does (TT-1).
        temporary_identity: false,
    };
    if !raw_birth.is_empty() && patient.birth_date.is_none() {
        warnings.push("PID-7 is not a date; birth date left blank".to_string());
    }
    if !raw_sex.is_empty() && patient.sex.is_none() {
        warnings.push("PID-8 is not a known sex code; left blank".to_string());
    }
    let mut problems = patients_store::validate(&patient);
    let death = reports_a_death(&get(&message, "PID-29"), &get(&message, "PID-30"))
        .then(|| death_occurred(&get(&message, "PID-29")));
    if !problems.is_empty() {
        if death.is_some() {
            problems.push(DEATH_NOT_APPLIED.to_string());
        }
        return Err(Rejection::Invalid(problems));
    }
    let message_time =
        message_time(&get(&message, "EVN-2")).or_else(|| message_time(&get(&message, "MSH-7")));
    Ok(Inbound {
        message_time,
        event,
        control_id: get(&message, "MSH-10"),
        patient,
        death,
        warnings,
    })
}

/// The `ACK` for a message, as ER7 text with `\r` between segments, or `None` when the input was
/// not a message and so has no sender to answer. `control_id` and `timestamp` (`YYYYMMDDHHMMSS`)
/// are the acknowledgement's own, supplied by the caller so this stays deterministic.
#[must_use]
pub fn acknowledgement(
    text: &str,
    outcome: &Result<Inbound, Rejection>,
    control_id: &str,
    timestamp: &str,
) -> Option<String> {
    let (code, note) = match outcome {
        Ok(_) => ("AA", None),
        Err(Rejection::NotHl7(why)) => ("AE", Some(why.clone())),
        Err(Rejection::Unsupported(why)) => ("AR", Some(why.clone())),
        Err(Rejection::Invalid(problems)) => ("AE", Some(problems.join("; "))),
        Err(Rejection::Kept(reasons)) => ("AR", Some(reasons.join("; "))),
    };
    let note = note.map(|n| msa_text(&n));
    let Ok(original) = hl7::v2::parse(text) else {
        // A frame that starts as an MSH but does not read: answer `AE`, echoing its MSH-10 when it can be found, so the
        // sender can match the answer (IH-2). Anything else has no sender to answer.
        return generic_nak(
            text,
            note.as_deref().unwrap_or(NOT_HL7),
            control_id,
            timestamp,
        );
    };
    let mut ack = hl7::v2::builder::acknowledge(&original, code, control_id, timestamp);
    // A message with no processing id is answered as production, `P`, as HL7 requires one in an ACK (IH-2).
    let processing = original.get("MSH-11.1").ok().flatten();
    let processing = processing.as_deref().unwrap_or("");
    if processing.is_empty() {
        ack = ack.processing_id("P");
    }
    if let Some(note) = note {
        ack = ack.set("MSA-3", &note);
    }
    ack.build()
        .ok()
        .map(|m| m.to_er7())
        .or_else(|| generic_nak(text, NOT_HL7, control_id, timestamp))
}

/// The longest `MSA-3` text sent (IH-2): HL7 v2.5 gives the field 80 characters and senders commonly allow more; the hub
/// this follows sends 199. Longer text is cut at a character boundary (OQ-IH5).
pub const MSA_TEXT_MAX: usize = 199;

/// `MSA-3` text: one line (every line break and tab a space, runs of spaces one), at most [`MSA_TEXT_MAX`] characters.
#[must_use]
pub fn msa_text(note: &str) -> String {
    let one_line = note
        .split(['\r', '\n', '\t', ' '])
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    one_line.chars().take(MSA_TEXT_MAX).collect()
}

/// HL7 v2's escapes for the standard delimiters, so text cannot break the message it is put in.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\E\\"),
            '|' => out.push_str("\\F\\"),
            '^' => out.push_str("\\S\\"),
            '&' => out.push_str("\\T\\"),
            '~' => out.push_str("\\R\\"),
            '\r' | '\n' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// An `AE` built by hand for a frame that starts with `MSH` and a separator but cannot be read: MSH-10 echoed when present
/// (cut to 199 characters and escaped), MSH-11 copied when it is `P`, `T` or `D` and `P` otherwise. `None` when the
/// text does not start as an MSH segment.
fn generic_nak(text: &str, note: &str, control_id: &str, timestamp: &str) -> Option<String> {
    let first = text.split(['\r', '\n']).next()?;
    let sep = first.strip_prefix("MSH")?.chars().next()?;
    if sep.is_alphanumeric() || sep.is_whitespace() {
        return None;
    }
    let fields: Vec<&str> = first.split(sep).collect();
    let control: String = fields
        .get(9)
        .copied()
        .unwrap_or("")
        .chars()
        .take(199)
        .collect();
    let processing = fields
        .get(10)
        .and_then(|f| f.chars().next())
        .filter(|c| matches!(c, 'P' | 'T' | 'D'))
        .unwrap_or('P');
    Some(format!(
        "MSH|^~\\&|||||{}||ACK|{}|{processing}|2.5\rMSA|AE|{}|{}",
        escape(timestamp),
        escape(control_id),
        escape(&control),
        escape(&msa_text(note))
    ))
}
#[cfg(test)]
mod tests {
    use super::*;

    const ADT: &str = "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261021090000||ADT^A28^ADT_A05|MSG1|P|2.5\r\
        EVN|A28|20261021090000\r\
        PID|1||9434765919^^^NHS^NH~M-1^^^LOCAL||SMITH^ANN^MARIE~JONES^ANN||19800314|F|||1 HIGH ST^^LEEDS^^LS1 4AB||01632 960001~^^^ann@example.org\r\
        ZXX|local segment";

    #[test]
    fn a_message_time_is_read_to_the_minute_with_or_without_an_offset() {
        let t = |s: &str| message_time(s).map(crate::clock::rfc3339_offset);
        assert_eq!(
            t("202611170930").as_deref(),
            Some("2026-11-17T09:30:00+00:00")
        );
        assert_eq!(
            t("20261117093015").as_deref(),
            Some("2026-11-17T09:30:15+00:00")
        );
        assert_eq!(
            t("20261117093015.1234").as_deref(),
            Some("2026-11-17T09:30:15+00:00")
        );
        assert_eq!(
            t("20261117093015+0100").as_deref(),
            Some("2026-11-17T08:30:15+00:00")
        );
        assert_eq!(
            t("20261117093015.5-0500").as_deref(),
            Some("2026-11-17T14:30:15+00:00")
        );
        for bad in [
            "",
            "2026",
            "20261117",
            "2026111709",
            "20261317093000",
            "20261117096000",
            "x0261117093000",
            "20261117093000+1",
            "20261117093000+2500",
        ] {
            assert_eq!(t(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_event_time_is_preferred_to_the_message_time() {
        let read = patient_from_adt(ADT).unwrap();
        assert_eq!(
            read.message_time
                .map(crate::clock::rfc3339_offset)
                .as_deref(),
            Some("2026-10-21T09:00:00+00:00")
        );
        let later_evn = ADT.replace("EVN|A28|20261021090000", "EVN|A28|20261021100000");
        let read = patient_from_adt(&later_evn).unwrap();
        assert_eq!(
            read.message_time
                .map(crate::clock::rfc3339_offset)
                .as_deref(),
            Some("2026-10-21T10:00:00+00:00")
        );
        let no_evn = ADT
            .replace("EVN|A28|20261021090000\r", "")
            .replace("|20261021090000||", "|||");
        assert_eq!(patient_from_adt(&no_evn).unwrap().message_time, None);
    }

    fn msa(ack: &str) -> Vec<String> {
        let line = ack.split('\r').find(|l| l.starts_with("MSA|")).unwrap();
        line.split('|').map(str::to_string).collect()
    }
    fn msh11(ack: &str) -> String {
        ack.split('\r')
            .next()
            .unwrap()
            .split('|')
            .nth(10)
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn an_acknowledgement_copies_the_processing_id_and_defaults_it_to_production() {
        // IH-2: a test message is never acknowledged as production.
        let ok = |text: &str| {
            acknowledgement(text, &patient_from_adt(text), "ACK1", "20261117090000").unwrap()
        };
        assert_eq!(msh11(&ok(ADT)), "P");
        assert_eq!(msh11(&ok(&ADT.replace("|MSG1|P|2.5", "|MSG1|T|2.5"))), "T");
        assert_eq!(msh11(&ok(&ADT.replace("|MSG1|P|2.5", "|MSG1||2.5"))), "P");
        assert_eq!(msa(&ok(ADT))[1..3], ["AA".to_string(), "MSG1".to_string()]);
    }

    #[test]
    fn msa_3_is_one_line_escaped_and_capped() {
        let long = format!("line one\r\nline\ttwo | a^b~c&d\\e {}", "x".repeat(400));
        let ack = acknowledgement(
            ADT,
            &Err(Rejection::Invalid(vec![long])),
            "ACK1",
            "20261117090000",
        )
        .unwrap();
        // The answer still reads as HL7, with the same two segments.
        let parsed = hl7::v2::parse(&ack).unwrap();
        assert_eq!(
            ack.split('\r').filter(|l| !l.is_empty()).count(),
            2,
            "{ack:?}"
        );
        let text = parsed.get("MSA-3").unwrap();
        let text = text.as_deref().unwrap_or("");
        assert!(
            text.starts_with("line one line two | a^b~c&d\\e x"),
            "{text:?}"
        );
        assert_eq!(text.chars().count(), MSA_TEXT_MAX);
        assert_eq!(msa_text("  a \r\n\r\n b\t\tc  "), "a b c");
    }

    #[test]
    fn an_msh_frame_that_does_not_read_gets_an_ae_with_its_control_id() {
        // IH-2: the sender can match the answer; the text is escaped, and what is not an MSH at all gets nothing.
        let broken = "MSH|^~\\&|A|B|C|D|20261117||ADT^A28|C9^9|T|2.5\rPID|\u{1}";
        let ack = acknowledgement(
            broken,
            &Err(Rejection::NotHl7(NOT_HL7.into())),
            "ACK1",
            "20261117090000",
        );
        if let Some(ack) = ack {
            let m = msa(&ack);
            assert_eq!(m[1], "AE", "{ack:?}");
            assert!(m[2].starts_with('C'), "the control id is echoed: {ack:?}");
            assert!(
                !ack.contains("9^9"),
                "a delimiter in MSH-10 is escaped: {ack:?}"
            );
            assert_eq!(msh11(&ack), "T");
        }
        let nak = generic_nak(
            "MSH|^~\\&|A|B|C|D|t||ADT^A28|C9|X|2.5",
            NOT_HL7,
            "ACK1",
            "20261117090000",
        )
        .unwrap();
        assert_eq!(
            nak,
            "MSH|^~\\&|||||20261117090000||ACK|ACK1|P|2.5\rMSA|AE|C9|not an HL7 v2 message: it does not start with a readable MSH segment"
        );
        assert!(generic_nak("hello", NOT_HL7, "1", "t").is_none());
        assert!(generic_nak("MSHA|B", NOT_HL7, "1", "t").is_none());
        let escaped = generic_nak("MSH|^~\\&||||||||id|with^caret", "a|b", "1", "t").unwrap();
        assert!(escaped.ends_with("MSA|AE|id|a\\F\\b"), "{escaped:?}");
    }

    #[test]
    fn a_finding_keeps_its_path_and_reason_and_loses_its_value() {
        assert_eq!(
            withhold_values(r#"error: PID[1]-7[1].1: "19801399" is not a valid DTM value"#),
            "error: PID[1]-7[1].1: (value withheld) is not a valid DTM value"
        );
        assert_eq!(withhold_values("no value here"), "no value here");
        assert_eq!(
            withhold_values(r#"a "b" c "d" e"#),
            "a (value withheld) c (value withheld) e"
        );
        assert_eq!(
            withhold_values(r#"cut "open to the end"#),
            "cut (value withheld)"
        );
    }

    #[test]
    fn an_admit_becomes_an_identity_record() {
        let i = patient_from_adt(ADT).unwrap();
        assert_eq!((i.event.as_str(), i.control_id.as_str()), ("A28", "MSG1"));
        let p = &i.patient;
        assert_eq!(p.names[0].family.as_deref(), Some("SMITH"));
        assert_eq!(p.names[0].given, ["ANN", "MARIE"]);
        assert_eq!(p.names[1].family.as_deref(), Some("JONES"));
        assert_eq!(p.birth_date.as_deref(), Some("1980-03-14"));
        assert_eq!(p.sex, Some(AdministrativeSex::Female));
        assert_eq!(
            p.identifiers,
            [
                Identifier {
                    system: NATIONAL_NUMBER_SYSTEM.into(),
                    value: "9434765919".into()
                },
                Identifier {
                    system: "urn:hl7v2:LOCAL".into(),
                    value: "M-1".into()
                },
            ]
        );
        assert_eq!(p.addresses[0].postal_code.as_deref(), Some("LS1 4AB"));
        assert_eq!(p.telecom.len(), 2);
        assert_eq!(
            p.telecom[0],
            Telecom {
                system: "phone".into(),
                value: "01632 960001".into()
            }
        );
        assert_eq!(
            p.telecom[1],
            Telecom {
                system: "email".into(),
                value: "ann@example.org".into()
            }
        );
        assert_eq!(p.id, None);
    }

    #[test]
    fn a_local_z_segment_does_not_stop_the_message() {
        assert!(patient_from_adt(ADT).is_ok());
    }

    #[test]
    fn dates_may_be_partial_and_carry_a_time() {
        assert_eq!(birth_date("1980"), Some("1980".into()));
        assert_eq!(birth_date("198003"), Some("1980-03".into()));
        assert_eq!(birth_date("19800314"), Some("1980-03-14".into()));
        assert_eq!(birth_date("198003141230+0100"), Some("1980-03-14".into()));
        assert_eq!(birth_date("80"), None);
        assert_eq!(birth_date(""), None);
    }

    #[test]
    fn a_bad_date_or_sex_is_left_blank_with_a_warning_not_guessed() {
        let text = ADT.replace("19800314|F", "March|X");
        let i = patient_from_adt(&text).unwrap();
        assert_eq!(i.patient.birth_date, None);
        assert_eq!(i.patient.sex, None);
        assert!(
            i.warnings.iter().any(|w| w.contains("PID-7")),
            "{:?}",
            i.warnings
        );
        assert!(
            i.warnings.iter().any(|w| w.contains("PID-8")),
            "{:?}",
            i.warnings
        );
    }

    #[test]
    fn an_identifier_without_an_authority_is_skipped_with_a_warning() {
        let text = ADT.replace("M-1^^^LOCAL", "M-1");
        let i = patient_from_adt(&text).unwrap();
        assert_eq!(i.patient.identifiers.len(), 1);
        assert!(
            i.warnings
                .iter()
                .any(|w| w.contains("no assigning authority"))
        );
    }

    #[test]
    fn escape_sequences_in_a_name_are_decoded() {
        let text = ADT.replace("SMITH^ANN^MARIE", "O'BRIEN \\T\\ SONS^ANN");
        let i = patient_from_adt(&text).unwrap();
        assert_eq!(i.patient.names[0].family.as_deref(), Some("O'BRIEN & SONS"));
    }

    #[test]
    fn the_services_identity_rules_still_apply() {
        // A bad national health number.
        let bad_number = ADT.replace("9434765919", "9434765910");
        assert!(
            matches!(patient_from_adt(&bad_number), Err(Rejection::Invalid(p)) if p.iter().any(|m| m.contains("modulus 11")))
        );
        // No family name.
        let nameless = ADT.replace("SMITH^ANN^MARIE~JONES^ANN", "");
        assert!(matches!(
            patient_from_adt(&nameless),
            Err(Rejection::Invalid(_))
        ));
        // No PID.
        let no_pid = "MSH|^~\\&|REG|C|EPR|H|20261021||ADT^A01|M2|P|2.5\rEVN|A01";
        assert_eq!(
            patient_from_adt(no_pid),
            Err(Rejection::Invalid(vec!["no PID segment".into()]))
        );
    }

    #[test]
    fn other_message_types_and_events_are_unsupported_and_garbage_is_not_hl7() {
        let oru = "MSH|^~\\&|LAB|L|EPR|H|20261021||ORU^R01|M3|P|2.5\rPID|1||1^^^X||A^B";
        assert!(matches!(
            patient_from_adt(oru),
            Err(Rejection::Unsupported(_))
        ));
        let discharge = ADT.replace("ADT^A28^ADT_A05", "ADT^A03");
        assert!(matches!(
            patient_from_adt(&discharge),
            Err(Rejection::Unsupported(_))
        ));
        assert!(matches!(
            patient_from_adt("hello"),
            Err(Rejection::NotHl7(_))
        ));
        assert!(matches!(patient_from_adt(""), Err(Rejection::NotHl7(_))));
    }

    #[test]
    fn each_outcome_gets_the_right_acknowledgement() {
        let ts = "20261021090001";
        let ok = patient_from_adt(ADT);
        let ack = acknowledgement(ADT, &ok, "ACK1", ts).unwrap();
        let parsed = er7::parse(&ack).unwrap();
        assert_eq!(parsed.query("MSA-1").unwrap().as_deref(), Some("AA"));
        assert_eq!(parsed.query("MSA-2").unwrap().as_deref(), Some("MSG1"));
        // The answer goes back where it came from.
        assert_eq!(parsed.query("MSH-5.1").unwrap().as_deref(), Some("REG"));
        assert_eq!(parsed.query("MSH-9.1").unwrap().as_deref(), Some("ACK"));

        let bad = ADT.replace("9434765919", "9434765910");
        let ack = acknowledgement(&bad, &patient_from_adt(&bad), "ACK2", ts).unwrap();
        let parsed = er7::parse(&ack).unwrap();
        assert_eq!(parsed.query("MSA-1").unwrap().as_deref(), Some("AE"));
        assert!(
            parsed
                .query("MSA-3")
                .unwrap()
                .unwrap()
                .contains("modulus 11")
        );

        let oru = "MSH|^~\\&|LAB|L|EPR|H|20261021||ORU^R01|M3|P|2.5\rPID|1";
        let ack = acknowledgement(oru, &patient_from_adt(oru), "ACK3", ts).unwrap();
        assert_eq!(
            er7::parse(&ack).unwrap().query("MSA-1").unwrap().as_deref(),
            Some("AR")
        );

        assert_eq!(
            acknowledgement("hello", &patient_from_adt("hello"), "ACK4", ts),
            None
        );
    }

    #[test]
    fn a_death_in_pid_29_or_30_is_read_as_a_report() {
        let message = |tail: &str| {
            format!(
                "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261021090000||ADT^A08^ADT_A01|C1|P|2.5\rEVN|A08|20261021090000\rPID|1||M1^^^LOCAL||Smith^Ann||19800314|F{tail}\r"
            )
        };
        let pipes = "|".repeat(21);
        let on = |at: &str| Occurred::Known { at: at.into() };
        let none = |reason: &str| Occurred::NotKnown {
            reason: reason.into(),
        };
        for (tail, death) in [
            (format!("{pipes}20261104"), on("2026-11-04")),
            (format!("{pipes}20261104|Y"), on("2026-11-04")),
            (format!("{pipes}20261104|N"), on("2026-11-04")),
            (format!("{pipes}202611"), on("2026-11")),
            (format!("{pipes}202611040930"), on("2026-11-04")),
            (format!("{pipes}|Y"), none("the message gave no date")),
            (format!("{pipes}soon|Y"), none(UNREADABLE_DEATH_DATE)),
        ] {
            assert_eq!(
                patient_from_adt(&message(&tail)).unwrap().death,
                Some(death)
            );
        }
        for tail in [
            String::new(),
            format!("{pipes}|N"),
            format!("{pipes}\"\"|\"\""),
        ] {
            assert_eq!(patient_from_adt(&message(&tail)).unwrap().death, None);
        }
        // A message that cannot be applied still says that it reported a death.
        let nameless = message(&format!("{pipes}20261104")).replace("Smith^Ann", "");
        match patient_from_adt(&nameless) {
            Err(Rejection::Invalid(why)) => assert!(why.contains(&DEATH_NOT_APPLIED.to_string())),
            other => panic!("{other:?}"),
        }
        // An event that is not handled, such as a discharge, says so too.
        let discharge = message(&format!("{pipes}20261104")).replace("ADT^A08", "ADT^A03");
        match patient_from_adt(&discharge) {
            Err(Rejection::Unsupported(why)) => assert!(why.contains(DEATH_NOT_APPLIED)),
            other => panic!("{other:?}"),
        }
    }
}

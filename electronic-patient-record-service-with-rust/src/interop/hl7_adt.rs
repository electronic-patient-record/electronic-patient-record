//! HL7 v2 patient-administration feeds (ADT) and their acknowledgements (crates `er7`, `hl7`).
//!
//! Real ADT feeds are rarely conformant, so the patient is read **leniently** with `er7` (a
//! missing optional segment or a local `Z` segment does not stop a registration), and the
//! dictionary check from `hl7::v2` is reported as `warnings` rather than as a refusal. What *does*
//! refuse a message is the identity rules the rest of the service already applies
//! (`patients_store::validate`): a patient with no family name or a bad NHS number is not
//! registered because a feed said so.
//!
//! The reply is an `ACK` built by `hl7::v2`: `AA` accepted, `AE` the message was understood but
//! its patient is unacceptable, `AR` the message type is not one this adapter handles.
//!
//! Not here: a network listener (MLLP, OR-3), storing the patient, matching it to an existing one,
//! or any segment other than `MSH`, `EVN` and `PID`.

use crate::patients_store;
use electronic_patient_record_matcher::{
    Address, AdministrativeSex, HumanName, Identifier, NHS_NUMBER_SYSTEM, Patient, Telecom,
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
    /// Things worth knowing that did not stop the message: dictionary findings, identifiers that
    /// were skipped.
    pub warnings: Vec<String>,
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
}

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

fn get(message: &er7::Message, path: &str) -> String {
    message
        .query(path)
        .ok()
        .flatten()
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

/// Read the patient from an ADT message.
///
/// # Errors
///
/// [`Rejection`], saying which acknowledgement (if any) is owed.
pub fn patient_from_adt(text: &str) -> Result<Inbound, Rejection> {
    let message = er7::parse(text).map_err(|e| Rejection::NotHl7(e.to_string()))?;
    let msg_type = get(&message, "MSH-9.1");
    let event = get(&message, "MSH-9.2");
    if msg_type != "ADT" || !HANDLED_EVENTS.contains(&event.as_str()) {
        return Err(Rejection::Unsupported(format!(
            "message type {msg_type}^{event} is not handled (ADT^{})",
            HANDLED_EVENTS.join(", ")
        )));
    }
    let mut warnings = Vec::new();
    // The dictionary check is advice here: parse again through the HL7 layer and keep its findings.
    match hl7::v2::parse(text) {
        Ok(m) => warnings.extend(m.validate().iter().map(ToString::to_string)),
        Err(e) => warnings.push(format!("hl7 v2: {e}")),
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
        if type_code == "NH" || authority.eq_ignore_ascii_case("NHS") {
            identifiers.push(Identifier {
                system: NHS_NUMBER_SYSTEM.into(),
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
    };
    if !raw_birth.is_empty() && patient.birth_date.is_none() {
        warnings.push(format!(
            "PID-7 {raw_birth:?} is not a date; birth date left blank"
        ));
    }
    if !raw_sex.is_empty() && patient.sex.is_none() {
        warnings.push(format!(
            "PID-8 {raw_sex:?} is not a known sex code; left blank"
        ));
    }
    let problems = patients_store::validate(&patient);
    if !problems.is_empty() {
        return Err(Rejection::Invalid(problems));
    }
    Ok(Inbound {
        event,
        control_id: get(&message, "MSH-10"),
        patient,
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
        Err(Rejection::NotHl7(_)) => return None,
        Err(Rejection::Unsupported(why)) => ("AR", Some(why.clone())),
        Err(Rejection::Invalid(problems)) => ("AE", Some(problems.join("; "))),
    };
    let original = hl7::v2::parse(text).ok()?;
    let mut ack = hl7::v2::builder::acknowledge(&original, code, control_id, timestamp);
    if let Some(note) = note {
        ack = ack.set("MSA-3", &note);
    }
    ack.build().ok().map(|m| m.to_er7())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADT: &str = "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261021090000||ADT^A28^ADT_A05|MSG1|P|2.5\r\
        EVN|A28|20261021090000\r\
        PID|1||9434765919^^^NHS^NH~M-1^^^LOCAL||SMITH^ANN^MARIE~JONES^ANN||19800314|F|||1 HIGH ST^^LEEDS^^LS1 4AB||01632 960001~^^^ann@example.org\r\
        ZXX|local segment";

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
                    system: NHS_NUMBER_SYSTEM.into(),
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
        // A bad NHS number.
        let bad_nhs = ADT.replace("9434765919", "9434765910");
        assert!(
            matches!(patient_from_adt(&bad_nhs), Err(Rejection::Invalid(p)) if p.iter().any(|m| m.contains("modulus 11")))
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
}

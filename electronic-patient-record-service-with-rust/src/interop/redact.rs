//! An HL7 v2 message with its values masked, for a log line (IH-4).
//!
//! Deny by default: the shape of a message stays visible (its segments, how many fields each has, which are empty)
//! and its values do not. In `MSH` the structural fields are kept: MSH-2 (the encoding characters), MSH-3 to MSH-6
//! (the sending and receiving application and facility), MSH-7 (the message time), MSH-9 (the type), MSH-11 (the
//! processing id) and MSH-12 (the version). MSH-8 (security) is masked, and so is MSH-10 (the control id), because
//! some senders build it from a patient's number (IH-3). Every other populated field, in every other segment, is
//! [`MASK`]; an empty field stays empty. Anything that does not start as an `MSH` segment with a readable separator is
//! masked whole. The separator is the fourth *character* of `MSH`, not the fourth byte, so text that is not ASCII
//! cannot split a character.

/// What a masked field, or a masked message, becomes.
pub const MASK: &str = "***";

/// MSH fields kept, by their HL7 number. MSH-1 is the separator itself.
const MSH_KEPT: [usize; 9] = [2, 3, 4, 5, 6, 7, 9, 11, 12];

/// `text` with every value masked except the structural `MSH` fields. Segments are separated by `\r` in the result.
#[must_use]
pub fn er7(text: &str) -> String {
    let mut segments = text.split(['\r', '\n']).filter(|s| !s.is_empty());
    let Some(first) = segments.next() else {
        return String::new();
    };
    let Some(sep) = first
        .strip_prefix("MSH")
        .and_then(|rest| rest.chars().next())
    else {
        return MASK.to_string();
    };
    if sep.is_alphanumeric() || sep.is_whitespace() {
        return MASK.to_string();
    }
    std::iter::once(msh(first, sep))
        .chain(segments.map(|s| other(s, sep)))
        .collect::<Vec<_>>()
        .join("\r")
}

/// `MSH` with only the kept fields: split index `i` (after the id) is MSH-`i + 1`, since MSH-1 is the separator.
fn msh(segment: &str, sep: char) -> String {
    segment
        .split(sep)
        .enumerate()
        .map(|(i, f)| {
            if i == 0 || f.is_empty() || i.checked_add(1).is_some_and(|n| MSH_KEPT.contains(&n)) {
                f
            } else {
                MASK
            }
        })
        .collect::<Vec<_>>()
        .join(&sep.to_string())
}

/// Any other segment: its id when it is one (three capital letters or digits), every populated field masked. A line
/// that is not a segment is masked whole.
fn other(segment: &str, sep: char) -> String {
    let mut fields = segment.split(sep);
    let id = fields.next().unwrap_or(""); // `split` always yields a first piece
    let is_id = id.len() == 3
        && id
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if !is_id {
        return MASK.to_string();
    }
    std::iter::once(id)
        .chain(fields.map(|f| if f.is_empty() { f } else { MASK }))
        .collect::<Vec<_>>()
        .join(&sep.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKER: &str = "QzMarkerQz";

    #[test]
    fn the_shape_and_the_structural_msh_fields_stay_and_every_value_goes() {
        let text = "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261117090000|SEC|ADT^A08^ADT_A05|20261117-9434765919|P|2.5\r\
                    EVN|A08|20261117090000\rPID|1||9434765919^^^NHS^NH||Smith^Ann||19800314|F\rZXX|local";
        assert_eq!(
            er7(text),
            "MSH|^~\\&|REG|CLINIC|EPR|HOSP|20261117090000|***|ADT^A08^ADT_A05|***|P|2.5\r\
             EVN|***|***\rPID|***||***||***||***|***\rZXX|***"
        );
    }

    #[test]
    fn what_is_not_er7_is_masked_whole() {
        assert_eq!(er7(""), "");
        for t in [
            "hello",
            "MSH",
            "MSHA|B",
            "MSH B|C",
            "PID|1||x",
            "\u{fffd}MSH|^~\\&",
        ] {
            assert_eq!(er7(t), MASK, "{t:?}");
        }
        // A line after MSH that is not a segment is masked whole too.
        assert_eq!(er7("MSH|^~\\&|A\rnot a segment|x"), "MSH|^~\\&|A\r***");
    }

    #[test]
    fn a_separator_that_is_not_ascii_is_read_as_one_character() {
        // A fuzz-found panic in the integration hub this follows: the fourth byte cut a character.
        assert_eq!(er7("MSH¦^~\\&¦A¦B\rPID¦1¦x"), "MSH¦^~\\&¦A¦B\rPID¦***¦***");
    }

    /// A small fixed generator, as in the feed's other fuzz tests: no dependency, and a failure repeats.
    struct Lcg(u64);
    impl Lcg {
        fn below(&mut self, n: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            usize::try_from(self.0 >> 33).unwrap_or(0) % n.max(1)
        }
    }

    #[test]
    fn whatever_the_text_it_never_panics_keeps_no_marker_and_masking_twice_changes_nothing() {
        // The marker is put anywhere except a kept MSH field's own text, damaged around, and must never survive.
        let parts = [
            "MSH", "|", "^", "~", "\\", "&", "\r", "\n", "PID", "EVN", "ZXX", MARKER, "", "1", "¦",
            "é", "\u{0}",
        ];
        let mut rng = Lcg(20_261_117);
        for _ in 0..20_000 {
            let mut text = String::new();
            for _ in 0..rng.below(40) {
                text.push_str(parts[rng.below(parts.len())]);
            }
            let once = er7(&text);
            assert_eq!(er7(&once), once, "not stable for {text:?}");
            // A marker can survive only inside a kept MSH field, which the generator can build by chance; check
            // the masked text with the kept MSH fields taken out.
            let mut masked = once.split('\r');
            let head = masked.next().unwrap_or("");
            let rest: Vec<&str> = masked.collect();
            assert!(
                !rest.iter().any(|s| s.contains(MARKER)),
                "{text:?} -> {once:?}"
            );
            if let Some(sep) = head.strip_prefix("MSH").and_then(|r| r.chars().next()) {
                for (i, f) in head.split(sep).enumerate() {
                    if i > 0 && !MSH_KEPT.contains(&(i + 1)) {
                        assert!(!f.contains(MARKER), "{text:?} -> {once:?}");
                    }
                }
            } else {
                assert!(!head.contains(MARKER), "{text:?} -> {once:?}");
            }
        }
    }
}

//! Record ids (CQ-9). Every id the service issues is a UUID, and a query binds it as `$1::uuid` so
//! the column's index is used. An id from a URL is checked here first: anything that is not a
//! hyphenated UUID is not found (as before), not a database error.

/// The lower-case form of a hyphenated UUID (`8-4-4-4-12` hex digits, any case), or `None`.
#[must_use]
pub fn uuid(value: &str) -> Option<String> {
    let groups: Vec<&str> = value.split('-').collect();
    let shape = [8, 4, 4, 4, 12];
    (groups.len() == shape.len()
        && groups
            .iter()
            .zip(shape)
            .all(|(g, n)| g.len() == n && g.bytes().all(|b| b.is_ascii_hexdigit())))
    .then(|| value.to_ascii_lowercase())
}

/// The UUIDs among `values`, lower-cased; the rest are dropped (none of them can match a record).
#[must_use]
pub fn uuids(values: &[String]) -> Vec<String> {
    values.iter().filter_map(|v| uuid(v)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uuid_in_any_case_is_lower_cased() {
        assert_eq!(
            uuid("44839249-B00A-4B9A-B092-675A7A42E324").as_deref(),
            Some("44839249-b00a-4b9a-b092-675a7a42e324")
        );
    }

    #[test]
    fn anything_else_is_none() {
        for bad in [
            "",
            "not-a-uuid",
            "44839249b00a4b9ab092675a7a42e324",
            "{44839249-b00a-4b9a-b092-675a7a42e324}",
            "44839249-b00a-4b9a-b092-675a7a42e32g",
            "44839249-b00a-4b9a-b092-675a7a42e324 ",
            "44839249-b00a-4b9a-b092-675a7a42e324-0",
            "4483924٩-b00a-4b9a-b092-675a7a42e324",
        ] {
            assert_eq!(uuid(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn uuids_drops_what_cannot_match() {
        let ids = vec![
            "nope".to_string(),
            "44839249-B00A-4B9A-B092-675A7A42E324".to_string(),
        ];
        assert_eq!(uuids(&ids), ["44839249-b00a-4b9a-b092-675a7a42e324"]);
    }
}

//! CQ-13: the FHIR controller renders what the services return; it does not call a JSON handler
//! and read its answer back. Database-free: it reads the source.

#[test]
fn fhir_calls_services_not_json_handlers() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/controllers/fhir.rs");
    let text = std::fs::read_to_string(path).unwrap();
    let code = text.split("#[cfg(test)]").next().unwrap_or_default();
    let mut found = Vec::new();
    for (n, line) in code.lines().enumerate() {
        let l = line.replace(' ', "");
        for banned in [
            "read_back(",
            "patients::read(",
            "patients::search(",
            "patients::create(",
            "patients::update(",
            "allergies::list(",
            "to_bytes(",
        ] {
            if l.contains(banned) {
                found.push(format!("fhir.rs:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}

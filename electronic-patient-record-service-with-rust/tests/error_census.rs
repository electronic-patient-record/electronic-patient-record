//! CQ-11: error bodies are built in one place, `src/api_error.rs`. A controller or middleware that
//! writes `{"error": …}` or `{"errors": …}` itself, or answers a refusal as plain text, drifts from
//! the shape the UI and the FHIR facade expect. Database-free: it reads the source.

use std::path::Path;

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_error_body_is_built_outside_api_error() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src.join("controllers"), &mut files);
    files.push(src.join("auth.rs"));
    files.push(src.join("authz.rs"));
    let mut found = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        // Unit tests may read bodies; only the code that answers requests is checked.
        let code = text.split("#[cfg(test)]").next().unwrap_or_default();
        for (n, line) in code.lines().enumerate() {
            let l = line.replace(' ', "");
            // The audit chain's verification report (424) carries `error` as one field of a report,
            // beside `intact` and `rows`; it is not an error answer.
            if l.contains("\"error\":\"key-not-held\"") {
                continue;
            }
            if l.contains("\"error\":") || l.contains("\"errors\":") || l.contains("json_error(") {
                found.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
            }
            if l.contains("(StatusCode::FORBIDDEN,\"") || l.contains("(status,msg).into_response()")
            {
                found.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "build errors with ApiError:\n{}",
        found.join("\n")
    );
}

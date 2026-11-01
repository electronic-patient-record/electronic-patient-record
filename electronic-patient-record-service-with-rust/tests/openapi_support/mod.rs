//! The small JSON Schema checker shared by the OpenAPI response tests (OS-7). It is strict where it
//! matters (required keys, keys outside `properties`, enums, types, `$ref`) and does not implement
//! `allOf`, `oneOf`, formats or length limits.
#![allow(dead_code)]

use serde_json::{Value, json};

pub fn resolve<'a>(doc: &'a Value, schema: &'a Value) -> &'a Value {
    match schema["$ref"].as_str() {
        Some(r) => {
            let name = r
                .strip_prefix("#/components/schemas/")
                .expect("a local ref");
            &doc["components"]["schemas"][name]
        }
        None => schema,
    }
}

pub fn type_ok(kind: &str, value: &Value) -> bool {
    match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => false,
    }
}

/// Problems found, each with the path to the value.
pub fn check(doc: &Value, schema: &Value, value: &Value, at: &str, out: &mut Vec<String>) {
    let schema = resolve(doc, schema);
    if let Some(words) = schema["enum"].as_array()
        && !words.contains(value)
    {
        out.push(format!("{at}: {value} is not one of {words:?}"));
        return;
    }
    match &schema["type"] {
        Value::String(k) if !type_ok(k, value) => {
            out.push(format!("{at}: expected {k}, got {value}"));
            return;
        }
        Value::Array(ks)
            if !ks
                .iter()
                .filter_map(Value::as_str)
                .any(|k| type_ok(k, value)) =>
        {
            out.push(format!("{at}: expected one of {ks:?}, got {value}"));
            return;
        }
        _ => {}
    }
    if let (Some(object), Some(props)) = (value.as_object(), schema["properties"].as_object()) {
        for key in schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !object.contains_key(key) {
                out.push(format!("{at}: lacks required {key}"));
            }
        }
        for (key, inner) in object {
            match props.get(key) {
                Some(s) => check(doc, s, inner, &format!("{at}.{key}"), out),
                None if schema["additionalProperties"] == json!(true) => {}
                None => out.push(format!("{at}: {key} is not in the schema")),
            }
        }
    }
    if let (Some(items), Some(item_schema)) = (value.as_array(), schema.get("items")) {
        for (i, item) in items.iter().enumerate() {
            check(doc, item_schema, item, &format!("{at}[{i}]"), out);
        }
    }
}

/// The schema a document gives for `method path` at `status`.
pub fn response_schema<'a>(doc: &'a Value, path: &str, method: &str, status: &str) -> &'a Value {
    let content = &doc["paths"][path][method]["responses"][status]["content"]["application/json"];
    assert!(
        !content.is_null(),
        "{method} {path} has no {status} body in the document"
    );
    &content["schema"]
}

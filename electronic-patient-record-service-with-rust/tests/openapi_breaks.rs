//! What a change to the API would break for somebody who already calls it (QA-7): the difference between two OpenAPI
//! documents, read from the caller's side. `openapi_census` refuses to bless a document that breaks the committed one
//! unless it is told that the break is meant.
//!
//! A caller **sends** parameters and request bodies, so the API may come to accept more and not less. A caller **gets**
//! answers, so the API may come to promise more and not less; a new word in a closed list of an answer is a break, since
//! a caller may have written out the list. It reads what the document says, and some of it is not said: the routes' own
//! refusals, the policy and the meaning of a field can change with no word here.

use serde_json::Value;
use std::collections::BTreeSet;

/// Which way a value travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Way {
    Sent,
    Got,
}

struct Compare<'a> {
    old: &'a Value,
    new: &'a Value,
    /// References already compared, so that a schema that holds itself ends.
    seen: BTreeSet<(String, String, Way)>,
    out: BTreeSet<String>,
}

/// Every break, each with where it is, sorted.
pub fn breaks(old: &Value, new: &Value) -> Vec<String> {
    let mut c = Compare {
        old,
        new,
        seen: BTreeSet::new(),
        out: BTreeSet::new(),
    };
    c.paths();
    c.out.into_iter().collect()
}

fn members(v: &Value) -> impl Iterator<Item = (&String, &Value)> {
    v.as_object().into_iter().flatten()
}

fn words(v: &Value) -> BTreeSet<String> {
    match v {
        Value::Array(all) => all.iter().map(Value::to_string).collect(),
        Value::Null => BTreeSet::new(),
        one => BTreeSet::from([one.to_string()]),
    }
}

/// The closed list of values, if there is one. A `const` is a list of one.
fn closed(v: &Value) -> Option<BTreeSet<String>> {
    v.get("enum")
        .map(words)
        .or_else(|| v.get("const").map(words))
}

impl<'a> Compare<'a> {
    fn say(&mut self, at: &str, what: &str) {
        self.out.insert(format!("{at}: {what}"));
    }

    fn paths(&mut self) {
        for (path, item) in members(&self.old["paths"]) {
            for (method, was) in members(item) {
                let at = format!("{} {path}", method.to_uppercase());
                match self.new["paths"].get(path).and_then(|i| i.get(method)) {
                    None => self.say(&at, "the operation is gone"),
                    Some(now) => self.operation(&at, was, now),
                }
            }
        }
    }

    fn operation(&mut self, at: &str, was: &'a Value, now: &'a Value) {
        if was["x-epr-public"] == true && now["x-epr-public"] != true {
            self.say(at, "it was public and needs a token");
        }
        self.parameters(at, was, now);
        let (body, next) = (&was["requestBody"], &now["requestBody"]);
        if next["required"] == true && body["required"] != true {
            self.say(at, "a request body is required");
        }
        self.content(&format!("{at} request"), body, next, Way::Sent);
        for (status, answer) in members(&was["responses"]) {
            let at = format!("{at} {status}");
            match now["responses"].get(status) {
                None => self.say(&at, "the answer is gone"),
                Some(next) => self.content(&at, answer, next, Way::Got),
            }
        }
    }

    fn parameters(&mut self, at: &str, was: &'a Value, now: &'a Value) {
        let empty = Vec::new();
        let list = |v: &'a Value| v["parameters"].as_array();
        let key = |p: &Value| (p["in"].to_string(), p["name"].to_string());
        let old = list(was).unwrap_or(&empty);
        let new = list(now).unwrap_or(&empty);
        for p in new {
            let before = old.iter().find(|q| key(q) == key(p));
            if p["required"] == true && before.is_none_or(|q| q["required"] != true) {
                self.say(&format!("{at} {}", p["name"]), "the parameter is required");
            }
        }
        for p in was["parameters"].as_array().into_iter().flatten() {
            let at = format!("{at} {}", p["name"]);
            let after = now["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|q| key(q) == key(p));
            match after {
                None => self.say(&at, "the parameter is gone"),
                Some(q) => self.schema(&at, &p["schema"], &q["schema"], Way::Sent),
            }
        }
    }

    fn content(&mut self, at: &str, was: &'a Value, now: &'a Value, way: Way) {
        for (kind, body) in members(&was["content"]) {
            match now["content"].get(kind) {
                None => self.say(at, &format!("{kind} is gone")),
                Some(next) => self.schema(at, &body["schema"], &next["schema"], way),
            }
        }
    }

    fn schema(&mut self, at: &str, was: &'a Value, now: &'a Value, way: Way) {
        let name = |v: &Value| v["$ref"].as_str().map(str::to_string);
        let (a, b) = (name(was), name(now));
        if a.is_some() || b.is_some() {
            let key = (
                a.clone().unwrap_or_else(|| was.to_string()),
                b.clone().unwrap_or_else(|| now.to_string()),
                way,
            );
            if !self.seen.insert(key) {
                return;
            }
            let (old, new) = (self.old, self.new);
            let follow = |doc: &'a Value, v: &'a Value, r: &Option<String>| match r {
                None => Some(v),
                Some(r) => doc.pointer(r.trim_start_matches('#')),
            };
            match (follow(old, was, &a), follow(new, now, &b)) {
                (Some(was), Some(now)) => {
                    let at = match &a {
                        Some(r) => r.rsplit('/').next().unwrap_or(at).to_string(),
                        None => at.to_string(),
                    };
                    self.schema(&at, was, now, way);
                }
                _ => self.say(at, "a schema it names is not there"),
            }
            return;
        }
        if was == now {
            // Alike here, and what it names may not be.
            self.inside(at, was, now, way);
            return;
        }
        self.values(at, was, now, way);
        self.limits(at, was, now, way);
        self.object(at, was, now, way);
        self.inside(at, was, now, way);
    }

    /// Types and closed lists.
    fn values(&mut self, at: &str, was: &Value, now: &Value, way: Way) {
        let side = match way {
            Way::Sent => "sent",
            Way::Got => "got",
        };
        let all = |v: &Value| v.get("type").map(words);
        for (what, old, new) in [
            ("type", all(was), all(now)),
            ("value", closed(was), closed(now)),
        ] {
            let lost: Vec<String> = match (way, &old, &new) {
                // What could be sent cannot; what comes back may be something it could not be.
                (Way::Sent, Some(o), Some(n)) => o.difference(n).cloned().collect(),
                (Way::Got, Some(o), Some(n)) => n.difference(o).cloned().collect(),
                (Way::Sent, None, Some(_)) | (Way::Got, Some(_), None) => {
                    vec![format!("any {what}")]
                }
                _ => Vec::new(),
            };
            for l in lost {
                let verb = match way {
                    Way::Sent => "is no longer taken",
                    Way::Got => "may come back",
                };
                self.say(at, &format!("the {what} {l} {verb} ({side})"));
            }
        }
    }

    /// Limits on what may be sent. An answer's limits are not a promise here.
    fn limits(&mut self, at: &str, was: &Value, now: &Value, way: Way) {
        if way == Way::Got {
            return;
        }
        for (name, tighter_is_less) in [
            ("maxLength", true),
            ("maximum", true),
            ("maxItems", true),
            ("minLength", false),
            ("minimum", false),
            ("minItems", false),
        ] {
            let tighter = match (was[name].as_f64(), now[name].as_f64()) {
                (None, Some(_)) => true,
                (Some(o), Some(n)) if tighter_is_less => n < o,
                (Some(o), Some(n)) => n > o,
                _ => false,
            };
            if tighter {
                self.say(at, &format!("{name} is tighter"));
            }
        }
        for name in ["pattern", "format"] {
            if now.get(name).is_some() && was.get(name) != now.get(name) {
                self.say(at, &format!("{name} changed"));
            }
        }
    }

    fn object(&mut self, at: &str, was: &'a Value, now: &'a Value, way: Way) {
        let required = |v: &Value| words(&v["required"]);
        let (old, new) = (required(was), required(now));
        match way {
            Way::Sent => {
                for p in new.difference(&old) {
                    self.say(at, &format!("{p} must be sent"));
                }
                if now["additionalProperties"] == false && was["additionalProperties"] != false {
                    self.say(at, "a field it does not know is refused");
                }
            }
            Way::Got => {
                for p in old.difference(&new) {
                    self.say(at, &format!("{p} may not come back"));
                }
            }
        }
        for (p, _) in members(&was["properties"]) {
            if now["properties"].get(p).is_none() {
                self.say(at, &format!("the field {p} is gone"));
            }
        }
    }

    /// What it holds.
    fn inside(&mut self, at: &str, was: &'a Value, now: &'a Value, way: Way) {
        for (p, old) in members(&was["properties"]) {
            if let Some(new) = now["properties"].get(p) {
                self.schema(&format!("{at}.{p}"), old, new, way);
            }
        }
        if was["items"].is_object() && now["items"].is_object() {
            self.schema(&format!("{at}[]"), &was["items"], &now["items"], way);
        }
        if was["additionalProperties"].is_object() && now["additionalProperties"].is_object() {
            let (a, b) = (&was["additionalProperties"], &now["additionalProperties"]);
            self.schema(&format!("{at}.*"), a, b, way);
        }
        for many in ["anyOf", "oneOf", "allOf"] {
            match (was[many].as_array(), now[many].as_array()) {
                (Some(a), Some(b)) if a.len() == b.len() => {
                    for (i, (x, y)) in a.iter().zip(b).enumerate() {
                        self.schema(&format!("{at}.{many}{i}"), x, y, way);
                    }
                }
                (None, None) => {}
                _ => self.say(at, &format!("{many} has other choices")),
            }
        }
    }
}

mod tests {
    use super::breaks;
    use serde_json::{Value, json};

    fn doc() -> Value {
        json!({
            "paths": {"/a/{id}": {
                "get": {"parameters": [
                        {"in": "path", "name": "id", "required": true, "schema": {"type": "string"}},
                        {"in": "query", "name": "q", "required": false, "schema": {"type": "string", "maxLength": 9, "enum": ["x", "y"]}}],
                    "responses": {"200": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Out"}}}}}},
                "post": {"x-epr-public": true, "requestBody": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/In"}}}},
                    "responses": {"201": {"description": "ok"}}}
            }},
            "components": {"schemas": {
                "In": {"type": "object", "required": ["a"], "properties": {"a": {"type": "string"}, "b": {"type": ["string", "null"]}}},
                "Out": {"type": "object", "required": ["a", "kids"], "properties": {
                    "a": {"enum": ["x", "y"]}, "b": {"type": "string"},
                    "kids": {"type": "array", "items": {"$ref": "#/components/schemas/Out"}}}}
            }}
        })
    }

    /// The breaks of one planted change.
    fn planted(pointer: &str, value: Value) -> Vec<String> {
        let old = doc();
        let mut new = doc();
        let (parent, last) = pointer.rsplit_once('/').unwrap();
        let parent = new.pointer_mut(parent).unwrap();
        match parent {
            Value::Object(map) if value.is_null() => {
                map.remove(last).unwrap();
            }
            Value::Object(map) => {
                map.insert(last.into(), value);
            }
            Value::Array(all) => all[last.parse::<usize>().unwrap()] = value,
            _ => panic!("{pointer}"),
        }
        assert_ne!(old, new, "{pointer} changed nothing");
        breaks(&old, &new)
    }

    const GET: &str = "/paths/~1a~1{id}/get";
    const IN: &str = "/components/schemas/In";
    const OUT: &str = "/components/schemas/Out";

    #[test]
    fn the_same_document_breaks_nothing_and_a_schema_that_holds_itself_ends() {
        assert!(breaks(&doc(), &doc()).is_empty());
        let real = electronic_patient_record::openapi::document();
        assert!(breaks(&real, &real).is_empty());
    }

    #[test]
    fn a_planted_break_is_found_and_says_where() {
        for (at, value, says) in [
            (
                GET.to_string(),
                Value::Null,
                "GET /a/{id}: the operation is gone",
            ),
            (
                format!("{GET}/responses/200"),
                Value::Null,
                "the answer is gone",
            ),
            (
                format!("{GET}/parameters/1/required"),
                json!(true),
                "the parameter is required",
            ),
            (
                format!("{GET}/parameters"),
                json!([doc()["paths"]["/a/{id}"]["get"]["parameters"][0]]),
                "\"q\": the parameter is gone",
            ),
            (
                format!("{GET}/parameters/1/schema/maxLength"),
                json!(8),
                "maxLength is tighter",
            ),
            (
                format!("{GET}/parameters/1/schema/enum"),
                json!(["x"]),
                "\"y\" is no longer taken",
            ),
            (
                "/paths/~1a~1{id}/post/x-epr-public".into(),
                json!(false),
                "needs a token",
            ),
            (
                "/paths/~1a~1{id}/post/requestBody/required".into(),
                json!(true),
                "a request body is required",
            ),
            (
                format!("{IN}/required"),
                json!(["a", "b"]),
                "In: \"b\" must be sent",
            ),
            (
                format!("{IN}/additionalProperties"),
                json!(false),
                "a field it does not know is refused",
            ),
            (
                format!("{IN}/properties/b"),
                Value::Null,
                "In: the field b is gone",
            ),
            (
                format!("{IN}/properties/b/type"),
                json!("string"),
                "\"null\" is no longer taken",
            ),
            (
                format!("{IN}/properties/a/pattern"),
                json!("^x$"),
                "pattern changed",
            ),
            (
                format!("{OUT}/properties/a/enum"),
                json!(["x", "y", "z"]),
                "\"z\" may come back",
            ),
            (
                format!("{OUT}/properties/a/enum"),
                Value::Null,
                "any value may come back",
            ),
            (
                format!("{OUT}/properties/b"),
                Value::Null,
                "Out: the field b is gone",
            ),
            (
                format!("{OUT}/properties/b/type"),
                json!(["string", "null"]),
                "\"null\" may come back",
            ),
            (
                format!("{OUT}/required"),
                json!(["kids"]),
                "\"a\" may not come back",
            ),
            (
                OUT.to_string(),
                Value::Null,
                "a schema it names is not there",
            ),
        ] {
            let found = planted(&at, value);
            assert!(found.iter().any(|f| f.contains(says)), "{at}: {found:?}");
        }
    }

    #[test]
    fn what_only_adds_is_not_a_break() {
        for (at, value) in [
            ("/paths/~1b".to_string(), json!({"get": {"responses": {}}})),
            (format!("{GET}/responses/404"), json!({"description": "no"})),
            (
                format!("{GET}/parameters/1/schema/enum"),
                json!(["x", "y", "z"]),
            ),
            (format!("{GET}/parameters/1/schema/maxLength"), json!(10)),
            (format!("{IN}/required"), json!([])),
            (format!("{IN}/properties/c"), json!({"type": "string"})),
            (format!("{IN}/properties/a/type"), json!(["string", "null"])),
            (format!("{OUT}/properties/c"), json!({"type": "string"})),
            (format!("{OUT}/required"), json!(["a", "b", "kids"])),
            (format!("{OUT}/properties/a/enum"), json!(["x"])),
            (format!("{OUT}/properties/b/description"), json!("words")),
        ] {
            let found = planted(&at, value);
            assert!(found.is_empty(), "{at}: {found:?}");
        }
    }
}

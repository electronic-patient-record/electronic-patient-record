//! Writing into a JSON value without a way to panic.
//!
//! `value["key"] = x` panics when `value` is a string, a number or an array. A response or an audit detail
//! is built from `serde_json::to_value`, which gives an object for a struct but is not promised to, so a write
//! goes through [`set`], which does nothing rather than panic when the value is not an object.

use serde_json::{Map, Value};

/// Set `key` to `value` in `target`: an object takes it, `null` becomes an object holding it, and anything
/// else is left as it is.
pub fn set(target: &mut Value, key: &str, value: Value) {
    if target.is_null() {
        *target = Value::Object(Map::new());
    }
    if let Value::Object(map) = target {
        map.insert(key.to_string(), value);
    }
}

/// Set the value at `path` (`["meta", "tag"]`), making an object of each step that is `null` or missing, and
/// leaving the value as it is when a step is something other than an object.
pub fn set_at(target: &mut Value, path: &[&str], value: Value) {
    match path {
        [] => *target = value,
        [key] => set(target, key, value),
        [key, rest @ ..] => {
            if target.is_null() {
                *target = Value::Object(Map::new());
            }
            if let Value::Object(map) = target {
                set_at(
                    map.entry((*key).to_string()).or_insert(Value::Null),
                    rest,
                    value,
                );
            }
        }
    }
}

/// Add the members of the object `other` to the object `target`; nothing happens unless both are objects.
pub fn extend(target: &mut Value, other: &Value) {
    if let (Value::Object(into), Value::Object(from)) = (target, other) {
        into.extend(from.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
}

/// Push `item` onto the array at `target[key]`; nothing happens when there is no such array.
pub fn push_at(target: &mut Value, key: &str, item: Value) {
    if let Some(list) = target.get_mut(key).and_then(Value::as_array_mut) {
        list.push(item);
    }
}

/// Whether `value` is an object whose `key` is the string `expected`.
#[must_use]
pub fn str_is(value: &Value, key: &str, expected: &str) -> bool {
    value.get(key).and_then(Value::as_str) == Some(expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_object_takes_the_key() {
        let mut v = json!({"a": 1});
        set(&mut v, "b", json!(2));
        assert_eq!(v, json!({"a": 1, "b": 2}));
        set(&mut v, "a", json!(3));
        assert_eq!(v, json!({"a": 3, "b": 2}));
    }

    #[test]
    fn a_path_makes_the_objects_it_needs_and_stops_at_one_that_is_not() {
        let mut v = json!({});
        set_at(&mut v, &["meta", "tag"], json!([1]));
        assert_eq!(v, json!({"meta": {"tag": [1]}}));
        set_at(&mut v, &[], json!(0));
        assert_eq!(v, json!(0));
        let mut s = json!({"meta": "text"});
        set_at(&mut s, &["meta", "tag"], json!([1]));
        assert_eq!(s, json!({"meta": "text"}));
    }

    #[test]
    fn extend_and_push_do_nothing_unless_the_shapes_fit() {
        let mut v = json!({"a": 1, "list": []});
        extend(&mut v, &json!({"b": 2}));
        push_at(&mut v, "list", json!("x"));
        push_at(&mut v, "a", json!("ignored"));
        push_at(&mut v, "missing", json!("ignored"));
        assert_eq!(v, json!({"a": 1, "b": 2, "list": ["x"]}));
        let mut s = json!("text");
        extend(&mut s, &json!({"b": 2}));
        extend(&mut v, &json!(5));
        assert_eq!(s, json!("text"));
        assert_eq!(v["b"], 2);
    }

    #[test]
    fn str_is_reads_without_indexing() {
        let v = json!({"a": "x", "n": 1});
        assert!(str_is(&v, "a", "x"));
        assert!(!str_is(&v, "a", "y"));
        assert!(!str_is(&v, "n", "1"));
        assert!(!str_is(&json!(3), "a", "x"));
    }

    #[test]
    fn null_becomes_an_object() {
        let mut v = Value::Null;
        set(&mut v, "k", json!(true));
        assert_eq!(v, json!({"k": true}));
    }

    #[test]
    fn a_value_that_is_not_an_object_is_left_alone_not_panicked_on() {
        for original in [json!("text"), json!(7), json!([1, 2]), json!(true)] {
            let mut v = original.clone();
            set(&mut v, "k", json!(1));
            assert_eq!(v, original);
        }
    }
}

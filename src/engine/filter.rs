use serde_json::Value;

pub fn remove_fields(value: &mut Value, fields: &[String]) {
    if fields.is_empty() {
        return;
    }
    remove_fields_impl(value, fields);
}

fn remove_fields_impl(value: &mut Value, fields: &[String]) {
    match value {
        Value::Object(map) => {
            let to_remove: Vec<String> = map
                .keys()
                .filter(|k| fields.iter().any(|f| f == *k || is_nested_match(f, k)))
                .cloned()
                .collect();

            for key in to_remove {
                map.remove(&key);
            }

            for (key, val) in map.iter_mut() {
                let nested_fields: Vec<String> = fields
                    .iter()
                    .filter_map(|f| strip_prefix(f, key))
                    .collect();
                if !nested_fields.is_empty() {
                    remove_fields_impl(val, &nested_fields);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                remove_fields_impl(item, fields);
            }
        }
        _ => {}
    }
}

pub fn include_fields(value: &mut Value, fields: &[String]) {
    if fields.is_empty() {
        return;
    }
    if let Value::Object(map) = value {
        let keys_to_keep: Vec<String> = map
            .keys()
            .filter(|k| fields.iter().any(|f| *f == **k))
            .cloned()
            .collect();
        map.retain(|k, _| keys_to_keep.contains(k));
    }
}

fn is_nested_match(field_path: &str, key: &str) -> bool {
    if let Some(pos) = field_path.find('.') {
        &field_path[..pos] == key
    } else {
        false
    }
}

fn strip_prefix(field_path: &str, prefix: &str) -> Option<String> {
    let full = format!("{prefix}.");
    if field_path.starts_with(&full) {
        Some(field_path[full.len()..].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_remove_top_level() {
        let mut v = json!({"name": "John", "password": "secret", "email": "john@test.com"});
        remove_fields(&mut v, &["password".to_string()]);
        assert_eq!(v, json!({"name": "John", "email": "john@test.com"}));
    }

    #[test]
    fn test_remove_nested() {
        let mut v = json!({
            "user": {"name": "John", "password": "secret"},
            "token": "abc"
        });
        remove_fields(&mut v, &["user.password".to_string(), "token".to_string()]);
        assert_eq!(v, json!({"user": {"name": "John"}}));
    }

    #[test]
    fn test_remove_in_array() {
        let mut v = json!([
            {"name": "John", "password": "s1"},
            {"name": "Jane", "password": "s2"}
        ]);
        remove_fields(&mut v, &["password".to_string()]);
        assert_eq!(
            v,
            json!([{"name": "John"}, {"name": "Jane"}])
        );
    }

    #[test]
    fn test_include_fields() {
        let mut v = json!({"name": "John", "email": "john@test.com", "password": "secret"});
        include_fields(&mut v, &["name".to_string(), "email".to_string()]);
        assert_eq!(v, json!({"name": "John", "email": "john@test.com"}));
    }

    #[test]
    fn test_empty_fields_noop() {
        let original = json!({"a": 1});
        let mut v = original.clone();
        remove_fields(&mut v, &[]);
        assert_eq!(v, original);
    }
}

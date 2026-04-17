use crate::config;
use serde_json::Value;

pub fn mask_fields(value: &mut Value, fields: &[String]) {
    if fields.is_empty() {
        return;
    }
    mask_fields_impl(value, fields);
}

fn mask_fields_impl(value: &mut Value, fields: &[String]) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                let direct_match = fields.iter().any(|f| f == key);
                if direct_match {
                    apply_mask(val);
                } else {
                    let nested_fields: Vec<String> = fields
                        .iter()
                        .filter_map(|f| {
                            let prefix = format!("{key}.");
                            f.strip_prefix(&prefix).map(|s| s.to_string())
                        })
                        .collect();
                    if !nested_fields.is_empty() {
                        mask_fields_impl(val, &nested_fields);
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                mask_fields_impl(item, fields);
            }
        }
        _ => {}
    }
}

fn apply_mask(value: &mut Value) {
    match value {
        Value::String(s) => {
            let masked = mask_string(s);
            *s = masked;
        }
        Value::Number(_) => {
            *value = Value::Number(serde_json::Number::from(0));
        }
        Value::Bool(_) => {
            *value = Value::Bool(false);
        }
        Value::Null => {}
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                apply_mask(item);
            }
        }
        Value::Object(map) => {
            for val in map.values_mut() {
                apply_mask(val);
            }
        }
    }
}

fn mask_string(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mask_char = config::MASK_CHAR;
    let display_len = config::DEFAULT_MASK_DISPLAY_LEN.min(s.len());

    if s.len() <= display_len {
        return mask_char.to_string().repeat(s.len());
    }

    let prefix_len = s.len() - display_len;
    let mut result = String::with_capacity(s.len());
    for _ in 0..prefix_len {
        result.push(mask_char);
    }
    result.push_str(&s[prefix_len..]);
    result
}

pub fn mask_value_completely(s: &str) -> String {
    config::MASK_CHAR.to_string().repeat(s.len().max(3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mask_string_field() {
        let mut v = json!({"token": "sk-1234567890abcdef", "name": "John"});
        mask_fields(&mut v, &["token".to_string()]);
        assert_eq!(v["token"], "********cdef");
        assert_eq!(v["name"], "John");
    }

    #[test]
    fn test_mask_short_string() {
        let mut v = json!({"token": "ab"});
        mask_fields(&mut v, &["token".to_string()]);
        assert_eq!(v["token"], "**");
    }

    #[test]
    fn test_mask_empty_string() {
        let mut v = json!({"token": ""});
        mask_fields(&mut v, &["token".to_string()]);
        assert_eq!(v["token"], "");
    }

    #[test]
    fn test_mask_nested() {
        let mut v = json!({
            "user": {"token": "secret123", "name": "John"}
        });
        mask_fields(&mut v, &["user.token".to_string()]);
        assert_eq!(v["user"]["token"], "*****t123");
        assert_eq!(v["user"]["name"], "John");
    }

    #[test]
    fn test_mask_in_array() {
        let mut v = json!([
            {"token": "abc123", "name": "A"},
            {"token": "def456", "name": "B"}
        ]);
        mask_fields(&mut v, &["token".to_string()]);
        assert_eq!(v[0]["token"], "***123");
        assert_eq!(v[1]["token"], "***456");
    }

    #[test]
    fn test_mask_number() {
        let mut v = json!({"credit_card": 1234567890});
        mask_fields(&mut v, &["credit_card".to_string()]);
        assert_eq!(v["credit_card"], 0);
    }
}

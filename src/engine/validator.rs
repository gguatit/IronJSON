use crate::error::{IronError, ValidationError};
use crate::rule::SchemaDef;
use serde_json::Value;

pub fn validate(value: &Value, schema: &SchemaDef) -> Result<(), IronError> {
    let mut errors = Vec::new();
    validate_impl(value, schema, "$", &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(IronError::Validation(errors))
    }
}

fn validate_impl(value: &Value, schema: &SchemaDef, path: &str, errors: &mut Vec<ValidationError>) {
    if let Some(ref cv) = schema.const_value {
        if value != cv {
            errors.push(ValidationError {
                path: path.to_string(),
                message: format!("Value must equal {cv}"),
                expected: cv.to_string(),
                found: value.to_string(),
            });
            return;
        }
    }

    if let Some(ref ev) = schema.enum_values {
        if !ev.iter().any(|e| e == value) {
            errors.push(ValidationError {
                path: path.to_string(),
                message: format!("Value must be one of {ev:?}"),
                expected: format!("one of: {ev:?}"),
                found: value.to_string(),
            });
            return;
        }
    }

    if !check_type(value, &schema.type_name) {
        errors.push(ValidationError {
            path: path.to_string(),
            message: format!(
                "Expected type '{}', got '{}'",
                schema.type_name,
                type_name(value)
            ),
            expected: schema.type_name.clone(),
            found: type_name(value),
        });
        return;
    }

    match (value, schema.type_name.as_str()) {
        (Value::Object(map), "object") => {
            for req in &schema.required {
                if !map.contains_key(req) {
                    errors.push(ValidationError {
                        path: format!("{path}.{req}"),
                        message: format!("Required field '{req}' is missing"),
                        expected: "present".to_string(),
                        found: "missing".to_string(),
                    });
                }
            }

            if let Some(false) = schema.additional_properties {
                let defined: std::collections::HashSet<&String> =
                    schema.properties.keys().collect();
                for key in map.keys() {
                    if !defined.contains(key) {
                        errors.push(ValidationError {
                            path: format!("{path}.{key}"),
                            message: format!("Additional property '{key}' is not allowed"),
                            expected: "only defined properties".to_string(),
                            found: format!("extra key: {key}"),
                        });
                    }
                }
            }

            for (key, val) in map {
                if let Some(prop_schema) = schema.properties.get(key) {
                    validate_impl(val, prop_schema, &format!("{path}.{key}"), errors);
                }
            }
        }
        (Value::String(s), "string") => {
            if let Some(min) = schema.min_length {
                let char_len = s.chars().count();
                if char_len < min {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("String length {} is less than minimum {}", char_len, min),
                        expected: format!("min_length: {min}"),
                        found: format!("length: {char_len}"),
                    });
                }
            }
            if let Some(max) = schema.max_length {
                let char_len = s.chars().count();
                if char_len > max {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("String length {} exceeds maximum {}", char_len, max),
                        expected: format!("max_length: {max}"),
                        found: format!("length: {char_len}"),
                    });
                }
            }
            if let Some(pattern) = &schema.pattern {
                if !s.contains(pattern.as_str()) {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("String does not contain pattern '{pattern}'"),
                        expected: format!("contains: {pattern}"),
                        found: format!("value: {s}"),
                    });
                }
            }
            if let Some(fmt) = &schema.format {
                validate_format(s, fmt, path, errors);
            }
        }
        (Value::Number(n), "number") | (Value::Number(n), "integer") => {
            if schema.type_name == "integer" && !n.is_i64() && !n.is_u64() {
                errors.push(ValidationError {
                    path: path.to_string(),
                    message: "Expected integer, got float".to_string(),
                    expected: "integer".to_string(),
                    found: "float".to_string(),
                });
            }
            if let (Some(min), Some(val)) = (schema.min, n.as_f64()) {
                if val < min {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("Value {val} is less than minimum {min}"),
                        expected: format!("min: {min}"),
                        found: format!("value: {val}"),
                    });
                }
            }
            if let (Some(max), Some(val)) = (schema.max, n.as_f64()) {
                if val > max {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("Value {val} exceeds maximum {max}"),
                        expected: format!("max: {max}"),
                        found: format!("value: {val}"),
                    });
                }
            }
        }
        (Value::Array(arr), "array") => {
            if let Some(min) = schema.min_items {
                if arr.len() < min {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("Array has {} items, minimum is {}", arr.len(), min),
                        expected: format!("min_items: {min}"),
                        found: format!("items: {}", arr.len()),
                    });
                }
            }
            if let Some(max) = schema.max_items {
                if arr.len() > max {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("Array has {} items, maximum is {}", arr.len(), max),
                        expected: format!("max_items: {max}"),
                        found: format!("items: {}", arr.len()),
                    });
                }
            }
            if let Some(true) = schema.unique_items {
                for i in 0..arr.len() {
                    for j in (i + 1)..arr.len() {
                        if arr[i] == arr[j] {
                            errors.push(ValidationError {
                                path: path.to_string(),
                                message: format!("Array contains duplicate at indices {i} and {j}"),
                                expected: "unique items".to_string(),
                                found: format!("duplicate: {}", arr[i]),
                            });
                            break;
                        }
                    }
                }
            }
            if let Some(item_schema) = &schema.items {
                for (i, item) in arr.iter().enumerate() {
                    validate_impl(item, item_schema, &format!("{path}[{i}]"), errors);
                }
            }
        }
        _ => {}
    }
}

fn validate_format(s: &str, fmt: &str, path: &str, errors: &mut Vec<ValidationError>) {
    let valid = match fmt {
        "email" => s.contains('@') && s.contains('.'),
        "uri" | "url" => s.starts_with("http://") || s.starts_with("https://"),
        "uuid" => {
            let parts: Vec<&str> = s.split('-').collect();
            parts.len() == 5
                && parts
                    .iter()
                    .all(|p| p.len() > 0 && p.chars().all(|c| c.is_ascii_hexdigit()))
        }
        "date" => {
            let parts: Vec<&str> = s.split('-').collect();
            parts.len() == 3 && parts[0].len() == 4 && parts[1].len() == 2 && parts[2].len() == 2
        }
        "ipv4" => {
            let parts: Vec<&str> = s.split('.').collect();
            parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok())
        }
        _ => true,
    };

    if !valid {
        errors.push(ValidationError {
            path: path.to_string(),
            message: format!("String does not match format '{fmt}'"),
            expected: format!("format: {fmt}"),
            found: format!("value: {s}"),
        });
    }
}

fn check_type(value: &Value, expected: &str) -> bool {
    match (value, expected) {
        (Value::Null, "null") => true,
        (Value::Bool(_), "boolean") => true,
        (Value::Number(_), "number") => true,
        (Value::Number(_), "integer") => true,
        (Value::String(_), "string") => true,
        (Value::Array(_), "array") => true,
        (Value::Object(_), "object") => true,
        _ => false,
    }
}

fn type_name(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(_) => "number".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Array(_) => "array".to_string(),
        Value::Object(_) => "object".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_schema(json: &str) -> SchemaDef {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn test_valid_object() {
        let schema = make_schema(
            r#"{
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {"type": "string", "min_length": 1}
            }
        }"#,
        );
        let value = json!({"name": "John"});
        assert!(validate(&value, &schema).is_ok());
    }

    #[test]
    fn test_missing_required() {
        let schema = make_schema(
            r#"{
            "type": "object",
            "required": ["name", "email"],
            "properties": {}
        }"#,
        );
        let value = json!({"name": "John"});
        let result = validate(&value, &schema);
        assert!(result.is_err());
        if let Err(IronError::Validation(errors)) = result {
            assert_eq!(errors.len(), 1);
            assert!(errors[0].path.contains("email"));
        } else {
            panic!("Expected validation error");
        }
    }

    #[test]
    fn test_string_min_length() {
        let schema = make_schema(r#"{"type": "string", "min_length": 3}"#);
        let value = json!("ab");
        let result = validate(&value, &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_number_range() {
        let schema = make_schema(r#"{"type": "integer", "min": 0, "max": 150}"#);
        assert!(validate(&json!(25), &schema).is_ok());
        assert!(validate(&json!(-1), &schema).is_err());
        assert!(validate(&json!(200), &schema).is_err());
    }

    #[test]
    fn test_array_validation() {
        let schema = make_schema(
            r#"{
            "type": "array",
            "min_items": 1,
            "max_items": 5,
            "items": {"type": "integer", "min": 0}
        }"#,
        );
        assert!(validate(&json!([1, 2, 3]), &schema).is_ok());
        assert!(validate(&json!([]), &schema).is_err());
        assert!(validate(&json!([1, -1, 3]), &schema).is_err());
    }

    #[test]
    fn test_string_pattern() {
        let schema = make_schema(r#"{"type": "string", "pattern": "@"}"#);
        assert!(validate(&json!("test@example.com"), &schema).is_ok());
        assert!(validate(&json!("no-email"), &schema).is_err());
    }

    #[test]
    fn test_wrong_type() {
        let schema = make_schema(r#"{"type": "string"}"#);
        let result = validate(&json!(123), &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_valid() {
        let schema = make_schema(r#"{"type": "string", "enum_values": ["active", "inactive"]}"#);
        assert!(validate(&json!("active"), &schema).is_ok());
        assert!(validate(&json!("inactive"), &schema).is_ok());
    }

    #[test]
    fn test_enum_invalid() {
        let schema = make_schema(r#"{"type": "string", "enum_values": ["active", "inactive"]}"#);
        let result = validate(&json!("pending"), &schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_const_valid() {
        let schema = make_schema(r#"{"type": "string", "const_value": "v1"}"#);
        assert!(validate(&json!("v1"), &schema).is_ok());
    }

    #[test]
    fn test_const_invalid() {
        let schema = make_schema(r#"{"type": "string", "const_value": "v1"}"#);
        assert!(validate(&json!("v2"), &schema).is_err());
    }

    #[test]
    fn test_additional_properties_false() {
        let schema = make_schema(
            r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "additional_properties": false
        }"#,
        );
        assert!(validate(&json!({"name": "John"}), &schema).is_ok());
        assert!(validate(&json!({"name": "John", "extra": 1}), &schema).is_err());
    }

    #[test]
    fn test_unique_items() {
        let schema =
            make_schema(r#"{"type": "array", "unique_items": true, "items": {"type": "integer"}}"#);
        assert!(validate(&json!([1, 2, 3]), &schema).is_ok());
        assert!(validate(&json!([1, 2, 1]), &schema).is_err());
    }

    #[test]
    fn test_format_email() {
        let schema = make_schema(r#"{"type": "string", "format": "email"}"#);
        assert!(validate(&json!("test@example.com"), &schema).is_ok());
        assert!(validate(&json!("not-an-email"), &schema).is_err());
    }

    #[test]
    fn test_format_ipv4() {
        let schema = make_schema(r#"{"type": "string", "format": "ipv4"}"#);
        assert!(validate(&json!("192.168.1.1"), &schema).is_ok());
        assert!(validate(&json!("999.999.999.999"), &schema).is_ok()); // u8 parse still works for each octet? No...
    }

    #[test]
    fn test_format_url() {
        let schema = make_schema(r#"{"type": "string", "format": "url"}"#);
        assert!(validate(&json!("https://example.com"), &schema).is_ok());
        assert!(validate(&json!("ftp://bad"), &schema).is_err());
    }

    #[test]
    fn test_string_min_length_multibyte() {
        let schema = make_schema(r#"{"type": "string", "min_length": 3}"#);
        assert!(validate(&json!("안녕하세요"), &schema).is_ok());
        assert!(validate(&json!("안녕"), &schema).is_err());
    }
}

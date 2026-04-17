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

fn validate_impl(
    value: &Value,
    schema: &SchemaDef,
    path: &str,
    errors: &mut Vec<ValidationError>,
) {
    if !check_type(value, &schema.type_name) {
        errors.push(ValidationError {
            path: path.to_string(),
            message: format!("Expected type '{}', got '{}'", schema.type_name, type_name(value)),
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

            for (key, val) in map {
                if let Some(prop_schema) = schema.properties.get(key) {
                    validate_impl(val, prop_schema, &format!("{path}.{key}"), errors);
                }
            }
        }
        (Value::String(s), "string") => {
            if let Some(min) = schema.min_length {
                if s.len() < min {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("String length {} is less than minimum {}", s.len(), min),
                        expected: format!("min_length: {min}"),
                        found: format!("length: {}", s.len()),
                    });
                }
            }
            if let Some(max) = schema.max_length {
                if s.len() > max {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: format!("String length {} exceeds maximum {}", s.len(), max),
                        expected: format!("max_length: {max}"),
                        found: format!("length: {}", s.len()),
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
            if let Some(item_schema) = &schema.items {
                for (i, item) in arr.iter().enumerate() {
                    validate_impl(item, item_schema, &format!("{path}[{i}]"), errors);
                }
            }
        }
        _ => {}
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
        let schema = make_schema(r#"{
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {"type": "string", "min_length": 1}
            }
        }"#);
        let value = json!({"name": "John"});
        assert!(validate(&value, &schema).is_ok());
    }

    #[test]
    fn test_missing_required() {
        let schema = make_schema(r#"{
            "type": "object",
            "required": ["name", "email"],
            "properties": {}
        }"#);
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
        let schema = make_schema(r#"{
            "type": "array",
            "min_items": 1,
            "max_items": 5,
            "items": {"type": "integer", "min": 0}
        }"#);
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
}

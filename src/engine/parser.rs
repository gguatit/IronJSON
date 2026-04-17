use crate::config;
use crate::error::IronError;

pub struct SafeParser {
    max_payload: usize,
    max_depth: usize,
}

impl SafeParser {
    pub fn new() -> Self {
        Self {
            max_payload: config::DEFAULT_MAX_PAYLOAD_BYTES,
            max_depth: config::MAX_JSON_DEPTH,
        }
    }

    pub fn with_limits(max_payload: usize, max_depth: usize) -> Self {
        Self {
            max_payload,
            max_depth,
        }
    }

    pub fn parse(&self, input: &[u8]) -> Result<serde_json::Value, IronError> {
        if input.len() > self.max_payload {
            return Err(IronError::PayloadTooLarge {
                actual: input.len(),
                limit: self.max_payload,
            });
        }

        let s = core::str::from_utf8(input)
            .map_err(|e| IronError::InvalidUtf8(e.to_string()))?;

        let trimmed = s.trim();
        if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            return Err(IronError::MalformedJson(
                "Input must be a JSON object or array".to_string(),
            ));
        }

        let value: serde_json::Value = serde_json::from_str(trimmed)?;

        self.validate_depth(&value, 0)?;

        self.validate_structure(&value)?;

        Ok(value)
    }

    pub fn parse_str(&self, input: &str) -> Result<serde_json::Value, IronError> {
        self.parse(input.as_bytes())
    }

    fn validate_depth(&self, value: &serde_json::Value, current: usize) -> Result<(), IronError> {
        if current > self.max_depth {
            return Err(IronError::MalformedJson(format!(
                "JSON nesting exceeds maximum depth of {}",
                self.max_depth
            )));
        }
        match value {
            serde_json::Value::Object(map) => {
                for (_k, v) in map {
                    self.validate_depth(v, current + 1)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    self.validate_depth(v, current + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn validate_structure(&self, value: &serde_json::Value) -> Result<(), IronError> {
        match value {
            serde_json::Value::Object(map) => {
                if map.len() > config::MAX_OBJECT_KEYS {
                    return Err(IronError::MalformedJson(format!(
                        "Object has {} keys, maximum is {}",
                        map.len(),
                        config::MAX_OBJECT_KEYS
                    )));
                }
                for (k, v) in map {
                    if k.len() > config::MAX_STRING_LEN {
                        return Err(IronError::MalformedJson(format!(
                            "Key '{:.20}...' exceeds maximum length",
                            k
                        )));
                    }
                    self.validate_structure(v)?;
                }
            }
            serde_json::Value::Array(arr) => {
                if arr.len() > config::MAX_ARRAY_ELEMENTS {
                    return Err(IronError::MalformedJson(format!(
                        "Array has {} elements, maximum is {}",
                        arr.len(),
                        config::MAX_ARRAY_ELEMENTS
                    )));
                }
                for v in arr {
                    self.validate_structure(v)?;
                }
            }
            serde_json::Value::String(s) => {
                if s.len() > config::MAX_STRING_LEN {
                    return Err(IronError::MalformedJson(format!(
                        "String value exceeds maximum length of {}",
                        config::MAX_STRING_LEN
                    )));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl Default for SafeParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_object() {
        let parser = SafeParser::new();
        let result = parser.parse_str(r#"{"name": "test", "age": 30}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_array() {
        let parser = SafeParser::new();
        let result = parser.parse_str(r#"[1, 2, 3]"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_payload_too_large() {
        let parser = SafeParser::with_limits(10, 64);
        let result = parser.parse_str(r#"{"key": "value that is too long"}"#);
        assert!(matches!(result, Err(IronError::PayloadTooLarge { .. })));
    }

    #[test]
    fn test_invalid_utf8() {
        let parser = SafeParser::new();
        let bad_bytes = b"\xff\xfe";
        let result = parser.parse(bad_bytes);
        assert!(matches!(result, Err(IronError::InvalidUtf8(_))));
    }

    #[test]
    fn test_malformed_not_json() {
        let parser = SafeParser::new();
        let result = parser.parse_str("not json");
        assert!(matches!(result, Err(IronError::MalformedJson(_))));
    }

    #[test]
    fn test_deep_nesting_rejected() {
        let parser = SafeParser::with_limits(1_000_000, 5);
        let deep = "{\"a\":".repeat(10) + &"{}".to_string() + &"}".repeat(10);
        let result = parser.parse_str(&deep);
        assert!(matches!(result, Err(IronError::MalformedJson(_))));
    }
}

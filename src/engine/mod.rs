pub mod filter;
pub mod mask;
pub mod parser;
pub mod transform;
pub mod validator;

use crate::error::IronError;
use crate::rule::{Direction, Rule, RuleEngine};

pub struct JsonEngine {
    rule_engine: RuleEngine,
    parser: parser::SafeParser,
}

impl JsonEngine {
    pub fn new(rules_json: Option<&str>) -> Result<Self, IronError> {
        let rule_engine = match rules_json {
            Some(json) => RuleEngine::from_json(json)?,
            None => RuleEngine::default()?,
        };
        Ok(Self {
            rule_engine,
            parser: parser::SafeParser::new(),
        })
    }

    pub fn process(
        &self,
        path: &str,
        method: &str,
        direction: Direction,
        body: &[u8],
    ) -> Result<serde_json::Value, IronError> {
        let mut value = self.parser.parse(body)?;

        let rules = self.rule_engine.find_matching_rules(path, method, direction);

        if rules.is_empty() {
            return Ok(value);
        }

        for rule in &rules {
            self.apply_rule(&mut value, rule)?;
        }

        Ok(value)
    }

    pub fn process_value(
        &self,
        path: &str,
        method: &str,
        direction: Direction,
        value: &mut serde_json::Value,
    ) -> Result<(), IronError> {
        let rules = self.rule_engine.find_matching_rules(path, method, direction);

        for rule in &rules {
            self.apply_rule(value, rule)?;
        }

        Ok(())
    }

    fn apply_rule(&self, value: &mut serde_json::Value, rule: &Rule) -> Result<(), IronError> {
        if let Some(schema) = &rule.validate {
            validator::validate(value, schema)?;
        }

        if !rule.remove_fields.is_empty() {
            filter::remove_fields(value, &rule.remove_fields);
        }

        if !rule.mask_fields.is_empty() {
            mask::mask_fields(value, &rule.mask_fields);
        }

        if !rule.rename.is_empty() {
            transform::rename_keys(value, &rule.rename);
        }

        if !rule.value_map.is_empty() {
            transform::apply_value_map(value, &rule.value_map);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_engine() -> JsonEngine {
        JsonEngine::new(None).unwrap()
    }

    #[test]
    fn test_process_removes_password() {
        let engine = test_engine();
        let body = r#"{"email": "test@example.com", "password": "secret123"}"#;
        let result = engine
            .process("/api/users", "POST", Direction::Request, body.as_bytes())
            .unwrap();
        assert!(result.get("password").is_none());
        assert_eq!(result["email"], "test@example.com");
    }

    #[test]
    fn test_process_masks_token() {
        let engine = test_engine();
        let body = r#"{"data": "ok", "token": "sk-secret-key-123"}"#;
        let result = engine
            .process("/api/data", "GET", Direction::Response, body.as_bytes())
            .unwrap();
        assert!(result["token"].as_str().unwrap().contains('*'));
    }

    #[test]
    fn test_process_validates_schema() {
        let engine = test_engine();
        let body = r#"{"name": "John"}"#;
        let result = engine.process("/api/users", "POST", Direction::Request, body.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_process_invalid_json() {
        let engine = test_engine();
        let body = b"not json at all";
        let result = engine.process("/api/users", "POST", Direction::Request, body);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_no_matching_rule() {
        let engine = test_engine();
        let body = r#"{"data": 123}"#;
        let result = engine
            .process("/health", "GET", Direction::Request, body.as_bytes())
            .unwrap();
        assert_eq!(result, json!({"data": 123}));
    }
}

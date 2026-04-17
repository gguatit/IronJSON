pub mod filter;
pub mod mask;
pub mod parser;
pub mod transform;
pub mod validator;

use crate::error::IronError;
use crate::rule::{Direction, Rule, RuleEngine, SchemaDef};

pub struct JsonEngine {
    rule_engine: RuleEngine,
    parser: parser::SafeParser,
}

struct MergedRule {
    validate: Option<SchemaDef>,
    remove_fields: Vec<String>,
    mask_fields: Vec<String>,
    rename: std::collections::HashMap<String, String>,
    value_map: std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
}

fn count_segments(path: &str) -> usize {
    path.split('/').filter(|s| !s.is_empty()).count()
}

fn merge_rules(rules: &[&Rule]) -> MergedRule {
    let mut validate_schema: Option<SchemaDef> = None;
    let mut remove_fields: Vec<String> = Vec::new();
    let mut mask_fields: Vec<String> = Vec::new();
    let mut rename: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut value_map: std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>> =
        std::collections::HashMap::new();

    for rule in rules {
        if rule.validate.is_some() && validate_schema.is_none() {
            validate_schema = rule.validate.clone();
        }
        for f in &rule.remove_fields {
            if !remove_fields.contains(f) {
                remove_fields.push(f.clone());
            }
        }
        for f in &rule.mask_fields {
            if !mask_fields.contains(f) {
                mask_fields.push(f.clone());
            }
        }
        for (k, v) in &rule.rename {
            rename.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for (k, v) in &rule.value_map {
            value_map.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }

    MergedRule {
        validate: validate_schema,
        remove_fields,
        mask_fields,
        rename,
        value_map,
    }
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

        let mut rules = self.rule_engine.find_matching_rules(path, method, direction);

        if rules.is_empty() {
            return Ok(value);
        }

        rules.sort_by(|a, b| {
            let a_s = count_segments(&a.path);
            let b_s = count_segments(&b.path);
            b_s.cmp(&a_s)
        });

        let merged = merge_rules(&rules);
        self.apply_rule(&mut value, &merged)?;

        Ok(value)
    }

    pub fn process_value(
        &self,
        path: &str,
        method: &str,
        direction: Direction,
        value: &mut serde_json::Value,
    ) -> Result<(), IronError> {
        let mut rules = self.rule_engine.find_matching_rules(path, method, direction);
        if !rules.is_empty() {
            rules.sort_by(|a, b| {
                let a_s = count_segments(&a.path);
                let b_s = count_segments(&b.path);
                b_s.cmp(&a_s)
            });
            let merged = merge_rules(&rules);
            self.apply_rule(value, &merged)?;
        }
        Ok(())
    }

    fn apply_rule(&self, value: &mut serde_json::Value, rule: &MergedRule) -> Result<(), IronError> {
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

    #[test]
    fn test_rule_merge_dedup() {
        let engine = test_engine();
        let body = r#"{"email":"a@b.com","password":"x","token":"sk-abc","secret":"y"}"#;
        let result = engine
            .process("/api/users", "POST", Direction::Request, body.as_bytes())
            .unwrap();
        assert!(result.get("password").is_none());
        assert!(result.get("secret").is_none());
        assert!(result["token"].as_str().unwrap().contains('*'));
    }
}

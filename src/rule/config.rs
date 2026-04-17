use crate::error::IronError;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub path: String,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default = "default_direction")]
    pub direction: Direction,
    #[serde(default)]
    pub validate: Option<SchemaDef>,
    #[serde(default)]
    pub remove_fields: Vec<String>,
    #[serde(default)]
    pub mask_fields: Vec<String>,
    #[serde(default)]
    pub rename: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub value_map: std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Request,
    Response,
    Both,
}

fn default_direction() -> Direction {
    Direction::Both
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchemaDef {
    #[serde(default = "default_schema_type")]
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub properties: std::collections::HashMap<String, SchemaDef>,
    #[serde(default)]
    pub items: Option<Box<SchemaDef>>,
    #[serde(default)]
    pub min_length: Option<usize>,
    #[serde(default)]
    pub max_length: Option<usize>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub min_items: Option<usize>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

fn default_schema_type() -> String {
    "object".to_string()
}

impl RuleSet {
    pub fn from_json(json: &str) -> Result<Self, IronError> {
        serde_json::from_str(json).map_err(|e| IronError::ConfigParse(e.to_string()))
    }

    pub fn default_rules() -> Result<Self, IronError> {
        Self::from_json(crate::config::DEFAULT_RULES_JSON)
    }
}

impl Rule {
    pub fn matches_method(&self, method: &str) -> bool {
        if self.methods.is_empty() {
            return true;
        }
        self.methods.iter().any(|m| m.eq_ignore_ascii_case(method))
    }

    pub fn matches_direction(&self, direction: Direction) -> bool {
        self.direction == Direction::Both || self.direction == direction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_rules() {
        let ruleset = RuleSet::default_rules().unwrap();
        assert_eq!(ruleset.rules.len(), 3);
        assert_eq!(ruleset.rules[0].path, "/api/users");
    }

    #[test]
    fn test_method_matching() {
        let mut rule = RuleSet::default_rules().unwrap().rules[0].clone();
        assert!(rule.matches_method("POST"));
        assert!(rule.matches_method("post"));
        assert!(!rule.matches_method("GET"));
    }

    #[test]
    fn test_direction_matching() {
        let rule = RuleSet::default_rules().unwrap().rules[0].clone();
        assert!(rule.matches_direction(Direction::Request));
        assert!(!rule.matches_direction(Direction::Response));
    }
}

pub mod config;
pub mod matcher;

pub use config::{Direction, Rule, RuleSet, SchemaDef};
pub use matcher::glob_match;

use crate::error::IronError;

pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    pub fn from_json(json: &str) -> Result<Self, IronError> {
        let ruleset = RuleSet::from_json(json)?;
        Ok(Self {
            rules: ruleset.rules,
        })
    }

    pub fn default() -> Result<Self, IronError> {
        let ruleset = RuleSet::default_rules()?;
        Ok(Self {
            rules: ruleset.rules,
        })
    }

    pub fn find_matching_rules(
        &self,
        path: &str,
        method: &str,
        direction: Direction,
    ) -> Vec<&Rule> {
        self.rules
            .iter()
            .filter(|rule| {
                matcher::glob_match(&rule.path, path)
                    && rule.matches_method(method)
                    && rule.matches_direction(direction)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_matching_rules() {
        let engine = RuleEngine::default().unwrap();
        let rules = engine.find_matching_rules("/api/users", "POST", Direction::Request);
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_no_matching_rules() {
        let engine = RuleEngine::default().unwrap();
        let rules = engine.find_matching_rules("/health", "GET", Direction::Request);
        assert!(rules.is_empty());
    }

    #[test]
    fn test_wildcard_match() {
        let engine = RuleEngine::default().unwrap();
        let rules = engine.find_matching_rules("/api/orders", "GET", Direction::Response);
        assert!(!rules.is_empty());
    }
}

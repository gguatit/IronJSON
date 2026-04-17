use std::fmt;

#[derive(Debug)]
pub enum IronError {
    JsonParse(String),
    Validation(Vec<ValidationError>),
    PayloadTooLarge { actual: usize, limit: usize },
    InvalidUtf8(String),
    NoMatchingRule(String),
    ConfigParse(String),
    Internal(String),
    MalformedJson(String),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
    pub expected: String,
    pub found: String,
}

impl fmt::Display for IronError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IronError::JsonParse(msg) => write!(f, "JSON parse error: {msg}"),
            IronError::Validation(errors) => {
                let details: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
                write!(f, "Validation failed: {}", details.join("; "))
            }
            IronError::PayloadTooLarge { actual, limit } => {
                write!(f, "Payload too large: {actual} bytes (limit: {limit})")
            }
            IronError::InvalidUtf8(msg) => write!(f, "Invalid UTF-8: {msg}"),
            IronError::NoMatchingRule(path) => write!(f, "No matching rule for path: {path}"),
            IronError::ConfigParse(msg) => write!(f, "Config parse error: {msg}"),
            IronError::Internal(msg) => write!(f, "Internal error: {msg}"),
            IronError::MalformedJson(msg) => write!(f, "Malformed JSON: {msg}"),
        }
    }
}

impl std::error::Error for IronError {}

impl IronError {
    pub fn to_response_json(&self) -> serde_json::Value {
        let (error_type, message) = match self {
            IronError::JsonParse(msg) => ("json_parse", msg.clone()),
            IronError::Validation(errors) => {
                return serde_json::json!({
                    "success": false,
                    "error": {
                        "type": "validation",
                        "message": "Validation failed",
                        "details": errors
                    }
                });
            }
            IronError::PayloadTooLarge { actual, limit } => (
                "payload_too_large",
                format!("Payload {actual} bytes exceeds limit of {limit} bytes"),
            ),
            IronError::InvalidUtf8(msg) => ("invalid_utf8", msg.clone()),
            IronError::NoMatchingRule(path) => (
                "no_matching_rule",
                format!("No rule matches path: {path}"),
            ),
            IronError::ConfigParse(msg) => ("config_parse", msg.clone()),
            IronError::Internal(msg) => ("internal", msg.clone()),
            IronError::MalformedJson(msg) => ("malformed_json", msg.clone()),
        };

        serde_json::json!({
            "success": false,
            "error": {
                "type": error_type,
                "message": message
            }
        })
    }

    pub fn http_status(&self) -> u16 {
        match self {
            IronError::JsonParse(_) | IronError::MalformedJson(_) => 400,
            IronError::Validation(_) => 422,
            IronError::PayloadTooLarge { .. } => 413,
            IronError::InvalidUtf8(_) => 400,
            IronError::NoMatchingRule(_) => 404,
            IronError::ConfigParse(_) => 500,
            IronError::Internal(_) => 500,
        }
    }
}

impl From<serde_json::Error> for IronError {
    fn from(e: serde_json::Error) -> Self {
        IronError::JsonParse(e.to_string())
    }
}

impl From<worker::Error> for IronError {
    fn from(e: worker::Error) -> Self {
        IronError::Internal(e.to_string())
    }
}

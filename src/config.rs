pub const DEFAULT_MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
pub const MASK_CHAR: char = '*';
pub const DEFAULT_MASK_DISPLAY_LEN: usize = 4;
pub const MAX_JSON_DEPTH: usize = 64;
pub const MAX_ARRAY_ELEMENTS: usize = 10_000;
pub const MAX_OBJECT_KEYS: usize = 1_000;
pub const MAX_STRING_LEN: usize = 1_000_000;

pub const DEFAULT_RULES_JSON: &str = r#"{
  "rules": [
    {
      "path": "/api/users",
      "methods": ["POST", "PUT", "PATCH"],
      "direction": "request",
      "validate": {
        "type": "object",
        "required": ["email"],
        "properties": {
          "email": { "type": "string", "min_length": 3 },
          "name": { "type": "string" },
          "age": { "type": "integer", "min": 0, "max": 200 }
        }
      },
      "remove_fields": ["password", "password_confirm", "secret"],
      "mask_fields": ["token", "api_key", "credit_card"],
      "rename": {},
      "value_map": {}
    },
    {
      "path": "/api/users/*",
      "methods": ["GET"],
      "direction": "response",
      "remove_fields": ["password", "internal_id"],
      "mask_fields": ["email", "phone"],
      "rename": { "internal_id": "id" },
      "value_map": {}
    },
    {
      "path": "/api/*",
      "methods": ["GET", "POST", "PUT", "PATCH", "DELETE"],
      "direction": "both",
      "mask_fields": ["token", "secret", "api_key"],
      "remove_fields": [],
      "rename": {},
      "value_map": {}
    }
  ]
}"#;

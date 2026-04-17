use serde_json::Value;

pub fn rename_keys(value: &mut Value, renames: &std::collections::HashMap<String, String>) {
    if renames.is_empty() {
        return;
    }
    rename_keys_impl(value, renames);
}

fn rename_keys_impl(value: &mut Value, renames: &std::collections::HashMap<String, String>) {
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for old_key in keys {
                let new_key_opt = renames.get(&old_key).cloned();
                if let Some(ref new_key) = new_key_opt {
                    if old_key != *new_key {
                        if let Some(val) = map.remove(&old_key) {
                            map.insert(new_key.clone(), val);
                        }
                    }
                }

                let nested_renames: std::collections::HashMap<String, String> = renames
                    .iter()
                    .filter_map(|(k, v)| {
                        let prefix = format!("{old_key}.");
                        k.strip_prefix(&prefix)
                            .map(|rest| (rest.to_string(), v.clone()))
                    })
                    .collect();

                if !nested_renames.is_empty() {
                    let lookup_key = new_key_opt.as_ref().unwrap_or(&old_key);
                    if let Some(nested) = map.get_mut(lookup_key) {
                        rename_keys_impl(nested, &nested_renames);
                    }
                }
            }

            let global_renames: std::collections::HashMap<String, String> = renames
                .iter()
                .filter(|(k, _)| !k.contains('.'))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            if !global_renames.is_empty() {
                for val in map.values_mut() {
                    rename_keys_impl(val, &global_renames);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                rename_keys_impl(item, renames);
            }
        }
        _ => {}
    }
}

pub fn apply_value_map(
    value: &mut Value,
    maps: &std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
) {
    if maps.is_empty() {
        return;
    }
    apply_value_map_impl(value, maps);
}

fn apply_value_map_impl(
    value: &mut Value,
    maps: &std::collections::HashMap<String, std::collections::HashMap<String, Value>>,
) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if let Some(mapping) = maps.get(key) {
                    if let Value::String(s) = val {
                        if let Some(new_val) = mapping.get(s) {
                            *val = new_val.clone();
                        }
                    }
                }

                let nested_maps: std::collections::HashMap<
                    String,
                    std::collections::HashMap<String, Value>,
                > = maps
                    .iter()
                    .filter_map(|(k, v)| {
                        let prefix = format!("{key}.");
                        k.strip_prefix(&prefix)
                            .map(|rest| (rest.to_string(), v.clone()))
                    })
                    .collect();

                if !nested_maps.is_empty() {
                    apply_value_map_impl(val, &nested_maps);
                } else {
                    let global_maps: std::collections::HashMap<
                        String,
                        std::collections::HashMap<String, Value>,
                    > = maps
                        .iter()
                        .filter(|(k, _)| !k.contains('.'))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    if !global_maps.is_empty() {
                        apply_value_map_impl(val, &global_maps);
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                apply_value_map_impl(item, maps);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rename_simple() {
        let mut v = json!({"old_name": "value", "keep": "data"});
        let mut renames = std::collections::HashMap::new();
        renames.insert("old_name".to_string(), "new_name".to_string());
        rename_keys(&mut v, &renames);
        assert_eq!(v, json!({"new_name": "value", "keep": "data"}));
    }

    #[test]
    fn test_rename_noop_empty() {
        let original = json!({"a": 1});
        let mut v = original.clone();
        rename_keys(&mut v, &std::collections::HashMap::new());
        assert_eq!(v, original);
    }

    #[test]
    fn test_value_map_simple() {
        let mut v = json!({"status": "active"});
        let mut maps = std::collections::HashMap::new();
        let mut status_map = std::collections::HashMap::new();
        status_map.insert("active".to_string(), json!(1));
        status_map.insert("inactive".to_string(), json!(0));
        maps.insert("status".to_string(), status_map);
        apply_value_map(&mut v, &maps);
        assert_eq!(v, json!({"status": 1}));
    }

    #[test]
    fn test_value_map_no_match() {
        let mut v = json!({"status": "pending"});
        let mut maps = std::collections::HashMap::new();
        let mut status_map = std::collections::HashMap::new();
        status_map.insert("active".to_string(), json!(1));
        maps.insert("status".to_string(), status_map);
        apply_value_map(&mut v, &maps);
        assert_eq!(v, json!({"status": "pending"}));
    }
}

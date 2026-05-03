use dashmap::DashMap;
use serde::Serialize;

/// CRUD for project-scoped JSON variables.

pub fn set_var(
    variables: &DashMap<String, serde_json::Value>,
    name: &str,
    value: serde_json::Value,
) {
    variables.insert(name.to_string(), value);
}

pub fn get_var(
    variables: &DashMap<String, serde_json::Value>,
    name: &str,
) -> Result<serde_json::Value, String> {
    variables
        .get(name)
        .map(|r| r.value().clone())
        .ok_or_else(|| format!("Variable '{}' not found", name))
}

#[derive(Debug, Serialize)]
pub struct VarSummary {
    pub name: String,
    pub value_type: String,
}

pub fn list_vars(variables: &DashMap<String, serde_json::Value>) -> Vec<VarSummary> {
    variables
        .iter()
        .map(|entry| VarSummary {
            name: entry.key().clone(),
            value_type: match entry.value() {
                serde_json::Value::Null => "null".to_string(),
                serde_json::Value::Bool(_) => "bool".to_string(),
                serde_json::Value::Number(_) => "number".to_string(),
                serde_json::Value::String(_) => "string".to_string(),
                serde_json::Value::Array(a) => format!("array[{}]", a.len()),
                serde_json::Value::Object(o) => format!("object{{{}}}", o.len()),
            },
        })
        .collect()
}

pub fn delete_var(
    variables: &DashMap<String, serde_json::Value>,
    name: &str,
) -> Result<(), String> {
    variables
        .remove(name)
        .map(|_| ())
        .ok_or_else(|| format!("Variable '{}' not found", name))
}

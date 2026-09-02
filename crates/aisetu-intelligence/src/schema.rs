//! Minimal JSON schema subset used by the intelligence contract.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use aisetu_core::SetuError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Object,
    String,
    Number,
    Boolean,
    Array,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: SchemaType,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub properties: indexmap_lite::Map,
}

mod indexmap_lite {
    use super::*;
    use std::collections::BTreeMap;

    pub type Map = BTreeMap<String, JsonSchema>;
}

impl JsonSchema {
    pub fn object(required: &[&str], properties: Vec<(&str, JsonSchema)>) -> Self {
        Self {
            schema_type: SchemaType::Object,
            required: required.iter().map(|s| (*s).to_string()).collect(),
            properties: properties
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    pub fn string() -> Self {
        Self {
            schema_type: SchemaType::String,
            required: Vec::new(),
            properties: Default::default(),
        }
    }

    pub fn number() -> Self {
        Self {
            schema_type: SchemaType::Number,
            required: Vec::new(),
            properties: Default::default(),
        }
    }

    pub fn boolean() -> Self {
        Self {
            schema_type: SchemaType::Boolean,
            required: Vec::new(),
            properties: Default::default(),
        }
    }

    pub fn validate(&self, value: &Value) -> aisetu_core::Result<()> {
        match self.schema_type {
            SchemaType::String => {
                if !value.is_string() {
                    return Err(SetuError::validation("expected string"));
                }
            }
            SchemaType::Number => {
                if !value.is_number() {
                    return Err(SetuError::validation("expected number"));
                }
            }
            SchemaType::Boolean => {
                if !value.is_boolean() {
                    return Err(SetuError::validation("expected boolean"));
                }
            }
            SchemaType::Array => {
                if !value.is_array() {
                    return Err(SetuError::validation("expected array"));
                }
            }
            SchemaType::Object => {
                let obj = value
                    .as_object()
                    .ok_or_else(|| SetuError::validation("expected object"))?;
                for key in &self.required {
                    if !obj.contains_key(key) {
                        return Err(SetuError::validation(format!(
                            "missing required field '{key}'"
                        )));
                    }
                }
                for (key, schema) in &self.properties {
                    if let Some(child) = obj.get(key) {
                        schema
                            .validate(child)
                            .map_err(|e| SetuError::validation(format!("field '{key}': {e}")))?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_required() {
        let schema = JsonSchema::object(
            &["name"],
            vec![
                ("name", JsonSchema::string()),
                ("age", JsonSchema::number()),
            ],
        );
        schema.validate(&json!({"name": "ada", "age": 36})).unwrap();
        assert!(schema.validate(&json!({"age": 36})).is_err());
        assert!(schema.validate(&json!({"name": 1})).is_err());
    }
}

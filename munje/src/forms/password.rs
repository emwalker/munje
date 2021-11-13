use serde::{Deserialize, Deserializer, Serialize};

use crate::{forms::Validate, types::Message};

#[derive(Debug, Default, Serialize)]
pub struct PasswordField {
    pub value: String,
    pub errors: Vec<String>,
}

impl PasswordField {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
            errors: Vec::new(),
        }
    }
}

impl Validate for PasswordField {
    fn validate(&mut self) -> bool {
        if self.value == "" {
            self.errors.push("Cannot be blank".to_string());
            return false;
        }

        true
    }

    fn is_valid(&self) -> bool {
        self.errors.len() == 0
    }

    fn messages(&self) -> Vec<Message> {
        self.errors
            .iter()
            .map(|string| Message {
                content: string.clone(),
                level: "error".to_string(),
            })
            .collect()
    }
}

impl Clone for PasswordField {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            errors: self.errors.clone(),
        }
    }
}

impl<'de> Deserialize<'de> for PasswordField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|t| PasswordField {
            value: t,
            errors: Vec::new(),
        })
    }
}

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
        if self.value.is_empty() {
            self.errors.push("Password cannot be empty".to_string());
            return false;
        }

        if self.value.len() < 8 {
            self.errors
                .push("Password must have at least eight characters".to_string());
            return false;
        }

        true
    }

    fn is_valid(&self) -> bool {
        self.errors.is_empty()
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

use serde::{Deserialize, Deserializer, Serialize};

use crate::{forms::Validate, types::Message};

#[derive(Debug, Default, Serialize)]
pub struct TextField {
    pub value: String,
    pub errors: Vec<String>,
}

impl TextField {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
            errors: Vec::new(),
        }
    }
}

impl Clone for TextField {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            errors: self.errors.clone(),
        }
    }
}

impl Validate for TextField {
    fn validate(&mut self) -> bool {
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

impl<'de> Deserialize<'de> for TextField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|t| TextField {
            value: t,
            errors: Vec::new(),
        })
    }
}

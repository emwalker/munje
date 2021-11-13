use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::convert::identity;

use crate::{
    forms::{PasswordField, TextField, Validate},
    models::UpsertResult,
    types::Pool,
    users::User,
};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct RegisterUser {
    pub handle: TextField,
    pub password: PasswordField,
    pub password_confirmation: PasswordField,
    is_valid: Option<bool>,
}

impl RegisterUser {
    pub async fn call(&self, db: &Pool) -> Result<UpsertResult<User>> {
        debug_assert_eq!(Some(true), self.is_valid);
        User::register(self, db).await
    }

    pub fn new(handle: &str, password: &str, password_confirmation: &str) -> Self {
        Self {
            handle: TextField::new(handle),
            password: PasswordField::new(password),
            password_confirmation: PasswordField::new(password_confirmation),
            is_valid: None,
        }
    }

    pub fn validate(&mut self) -> bool {
        if let Some(valid) = self.is_valid {
            return valid;
        }

        let mut valid = vec![
            self.handle.validate(),
            self.password.validate(),
            self.password_confirmation.validate(),
        ];

        if self.handle.value.len() < 3 {
            self.handle
                .errors
                .push("Must have at least three characters".to_string());
            valid.push(false);
        }

        if self.password.value != self.password_confirmation.value {
            self.password_confirmation
                .errors
                .push("Passwords do not match".to_string());
            valid.push(false);
        }

        let valid = valid.into_iter().all(identity);
        self.is_valid = Some(valid);
        valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_includes(errors: Vec<String>, string: &str) {
        let string = string.to_string();
        assert!(
            errors.clone().into_iter().any(|s| s == string),
            "collection does not contain string: {:?}",
            errors
        );
    }

    #[test]
    fn invalid_if_password_mismatch() {
        let mut form = RegisterUser::new("gnusto", "pass1", "pass2");

        assert!(!form.validate());
        assert!(!form.password_confirmation.is_valid());
        assert_includes(form.password_confirmation.errors, "Passwords do not match");
    }

    #[test]
    fn invalid_if_handle_not_long_enough() {
        let mut form = RegisterUser::new("gn", "pass1", "pass1");

        assert!(!form.validate());
        assert!(!form.handle.is_valid());
        assert_includes(form.handle.errors, "Must have at least three characters");
    }

    #[test]
    fn invalid_if_password_blank() {
        let mut form = RegisterUser::new("gnusto", "", "");

        assert!(!form.validate());
        assert!(!form.password.is_valid());
        assert_includes(form.password.errors, "Cannot be blank");
    }
}

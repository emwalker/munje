use serde::{Deserialize, Serialize};
use std::convert::identity;

use crate::{
    forms::{PasswordField, TextField, Validate},
    models::UpsertResult,
    prelude::*,
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
    pub async fn call(
        &self,
        request: &HttpRequest,
        db: &Pool,
    ) -> Result<UpsertResult<User>, Error> {
        debug_assert_eq!(Some(true), self.is_valid);
        let result = User::register(self, db).await?;
        request.set_user(result.record.clone())?;
        Ok(result)
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
                .push("Username must have at least three characters".to_string());
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

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AuthenticateUser {
    pub handle: TextField,
    pub password: PasswordField,
    is_valid: Option<bool>,
}

impl AuthenticateUser {
    #[allow(dead_code)]
    pub fn new(handle: &str, password: &str) -> Self {
        Self {
            handle: TextField::new(handle),
            password: PasswordField::new(password),
            is_valid: None,
        }
    }

    pub async fn call(&self, request: &HttpRequest, db: &Pool) -> Result<(), Error> {
        debug_assert_eq!(Some(true), self.is_valid);
        let user = User::authenticate(self, db).await?;
        User::update_last_login(user.id, db).await?;
        request.set_user(user)?;
        Ok(())
    }

    pub fn validate(&mut self) -> bool {
        if let Some(valid) = self.is_valid {
            return valid;
        }

        let mut valid = vec![self.handle.validate(), self.password.validate()];

        if self.handle.value.len() < 1 {
            self.handle
                .errors
                .push("Username cannot be empty".to_string());
            valid.push(false);
        }

        if self.password.value.len() < 1 {
            self.password
                .errors
                .push("Password cannot be empty".to_string());
            valid.push(false);
        }

        let valid = valid.into_iter().all(identity);
        self.is_valid = Some(valid);
        valid
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DestroyUserSession;

impl DestroyUserSession {
    pub async fn call(&self, request: &HttpRequest) -> Result<(), Error> {
        use actix_session::UserSession;
        request.get_session().clear();
        Ok(())
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
    fn register_user_invalid_if_password_mismatch() {
        let mut mutation = RegisterUser::new("gnusto", "pass1", "pass2");

        assert!(!mutation.validate());
        assert!(!mutation.password_confirmation.is_valid());
        assert_includes(
            mutation.password_confirmation.errors,
            "Passwords do not match",
        );
    }

    #[test]
    fn register_user_invalid_if_handle_not_long_enough() {
        let mut mutation = RegisterUser::new("gn", "pass1", "pass1");

        assert!(!mutation.validate());
        assert!(!mutation.handle.is_valid());
        assert_includes(
            mutation.handle.errors,
            "Username must have at least three characters",
        );
    }

    #[test]
    fn register_user_invalid_if_password_blank() {
        let mut mutation = RegisterUser::new("gnusto", "", "");

        assert!(!mutation.validate());
        assert!(!mutation.password.is_valid());
        assert_includes(mutation.password.errors, "Password cannot be empty");
    }

    #[test]
    fn authenticate_user_invalid_if_username_blank() {
        let mut mutation = AuthenticateUser::new("", "pass1");

        assert!(!mutation.validate());
        assert!(!mutation.handle.is_valid());
        assert_includes(mutation.handle.errors, "Username cannot be empty");
    }

    #[test]
    fn authenticate_user_invalid_if_password_blank() {
        let mut mutation = AuthenticateUser::new("gnusto", "");

        assert!(!mutation.validate());
        assert!(!mutation.password.is_valid());
        assert_includes(mutation.password.errors, "Password cannot be empty");
    }
}

use actix_session::UserSession;
use actix_web::HttpRequest;

use crate::error::Error;
use crate::users::User;

/// `Authentication` is kind of a request guard - it returns a Future which will resolve
/// with either the current authenticated user, or "error" out if the user has no session data
/// that'd tie them to a user profile, or if the session cache can't be read, or if the database
/// has issues, or... pick your poison I guess.
///
pub trait Authentication {
    /// Returns whether a user session exists and is valid.
    fn is_authenticated(&self) -> Result<bool, Error>;

    /// Sets a serializable user instance.
    fn set_user(&self, account: User) -> Result<(), Error>;

    /// Returns a User, if it can be extracted properly.
    fn user(&self) -> Result<User, Error>;
}

impl Authentication for HttpRequest {
    #[inline(always)]
    fn is_authenticated(&self) -> Result<bool, Error> {
        Ok(self
            .get_session()
            .get::<serde_json::Value>("user")?
            .is_some())
    }

    fn set_user(&self, user: User) -> Result<(), Error> {
        info!("Setting user session for {}", user.handle);
        self.get_session().insert("user", user)?;
        Ok(())
    }

    fn user(&self) -> Result<User, Error> {
        match self.get_session().get("user")? {
            Some(user) => {
                info!("User session found for {:?}", user);
                Ok(user)
            }
            None => {
                info!("No session found, page interaction will be anonymous");
                Ok(User::guest())
            }
        }
    }
}

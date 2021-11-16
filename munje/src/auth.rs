use actix_identity::Identity;

use crate::prelude::*;

pub fn user(id: &Identity) -> Result<User, Error> {
    let string = id.identity().ok_or(Error::Unauthorized)?;
    serde_json::from_str(&string).map_err(|e| {
        error!("Unable to deserialize user: {:?}", e);
        Error::Unauthorized
    })
}

pub fn user_or_guest(id: &Identity) -> Result<User, Error> {
    let user = match id.identity() {
        Some(string) => serde_json::from_str(&string)?,
        None => User::guest(),
    };
    Ok(user)
}

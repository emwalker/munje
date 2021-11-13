mod password;
mod text;
use crate::types::Message;
pub use password::PasswordField;
pub use text::TextField;

pub trait Validate {
    fn validate(&mut self) -> bool;

    fn is_valid(&self) -> bool;

    fn messages(&self) -> Vec<Message>;
}

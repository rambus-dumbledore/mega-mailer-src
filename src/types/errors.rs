use std::fmt;
use std::error;

#[derive(Debug, Clone)]
pub struct UserNotRegisteredError;
impl fmt::Display for UserNotRegisteredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User is not registered")
    }
}
impl error::Error for UserNotRegisteredError {}

#[derive(Debug, Clone)]
pub struct AuthCodeInvalidError;
impl fmt::Display for AuthCodeInvalidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Authorization code is invalid")
    }
}
impl error::Error for AuthCodeInvalidError {}
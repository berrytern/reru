use std::error::Error as StdError;

use pyo3::{PyErr};
use pyo3::exceptions::{PyException};
use thiserror::Error;


#[derive(Error, Debug)]
pub struct ReError {
    pub message: String,
}

impl std::fmt::Display for ReError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<ReError> for AppError {
    fn from(error: ReError) -> Self {
        AppError::RegexError(error)
    }
}

pub enum AppError{
    RegexError(ReError),
    InvalidPattern(ReError),
    IndexOutOfBounds(ReError),
}


impl StdError for AppError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            AppError::RegexError(e) => Some(e),
            AppError::InvalidPattern(e) => Some(e),
            AppError::IndexOutOfBounds(e) => Some(e),
        }
    }
}
impl std::fmt::Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self}")?;
        if let Some(e) = self.source() {
            writeln!(f, "\tCaused by: {e:?}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for AppError { // Error message for users.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use AppError::*;
        let message = match self {
            RegexError(_) => "Failed to read the file.",
            InvalidPattern(msg) =>  &msg.message,
            IndexOutOfBounds(msg) => &msg.message,
        };
        write!(f, "{message}")
    }
}

impl From<AppError> for PyErr {
    fn from(error: AppError) -> Self {
        PyException::new_err(format!("{error}"))
    }
}



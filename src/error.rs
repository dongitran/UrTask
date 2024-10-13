use std::error::Error;
use std::fmt;
use bson::ser::Error as BsonSerError;

#[derive(Debug)]
pub struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for AppError {}

impl From<String> for AppError {
    fn from(error: String) -> Self {
        AppError(error)
    }
}

impl From<mongodb::error::Error> for AppError {
    fn from(error: mongodb::error::Error) -> Self {
        AppError(error.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError(error.to_string())
    }
}

// Thêm implementation mới này
impl From<BsonSerError> for AppError {
    fn from(error: BsonSerError) -> Self {
        AppError(error.to_string())
    }
}
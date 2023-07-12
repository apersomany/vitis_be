use std::fmt::{Debug, Formatter};

use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;

pub mod resource;
pub mod search;
pub mod series;
pub mod single;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for Error {
    fn from(value: E) -> Self {
        Self(value.into())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let error = (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        );
        error.into_response()
    }
}

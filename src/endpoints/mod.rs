use std::fmt::{Debug, Formatter};

use anyhow::anyhow;
use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;

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

fn get_param(url: &str, key: &str) -> Result<String> {
    fn inner(url: &str, key: &str) -> Option<String> {
        let val = url
            .split(&format!("{key}="))
            .nth(1)?
            .split("&")
            .nth(0)?
            .to_string();
        Some(val)
    }
    if let Some(val) = inner(url, key) {
        Ok(val)
    } else {
        Err(anyhow!("could not get param {key} in {url}"))?
    }
}

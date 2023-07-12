use anyhow::{anyhow, Result};

pub fn get_param(url: &str, key: &str) -> Result<String> {
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

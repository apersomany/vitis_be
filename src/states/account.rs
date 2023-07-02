use std::sync::{Arc, Mutex, OnceLock};

use axum::http::HeaderMap;
use chrono::NaiveDateTime;
use cookie::Cookie;
use dashmap::DashMap;
use rand::random;
use reqwest::{cookie::CookieStore, header::HeaderValue, Client, Proxy, Url};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Account {
    token: Arc<Token>,
    #[serde(default = "generate_agent")]
    agent: String,
    proxy: Option<String>,
    pub gotchas: DashMap<i32, NaiveDateTime>,
    pub balance: i32,
    #[serde(skip)]
    client: OnceLock<Client>,
}

#[derive(Serialize, Deserialize)]
struct Token(Mutex<String>);

impl CookieStore for Token {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        if url.host_str() == Some("page.kakao.com") {
            for cookie_header in cookie_headers {
                let cookie = Cookie::parse(cookie_header.to_str().unwrap()).unwrap();
                if cookie.name() == "_kpwtkn" {
                    println!("{}", cookie.value());
                    *self.0.lock().unwrap() = cookie.value().to_string();
                }
            }
        }
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        if url.host_str() == Some("page.kakao.com") {
            let cookie_header = format!("_kpwtkn={}", self.0.lock().unwrap())
                .parse()
                .unwrap();
            Some(cookie_header)
        } else {
            None
        }
    }
}

pub fn generate_agent() -> String {
    format!("kakaopage/{:16x}", random::<u64>())
}

impl Account {
    pub fn client(&self) -> &Client {
        self.client.get_or_init(|| {
            let mut headers = HeaderMap::new();
            headers.insert("referer", "https://page.kakao.com".parse().unwrap());
            let mut builder = Client::builder()
                .cookie_provider(self.token.clone())
                .cookie_store(true)
                .default_headers(headers)
                .user_agent(&self.agent);
            if let Some(proxy) = &self.proxy {
                builder = builder.proxy(Proxy::http(proxy).unwrap())
            }
            builder.build().unwrap()
        })
    }
}

use std::sync::Arc;

use anyhow::{anyhow, Result};
use reqwest::header::HeaderMap;
use search::vars::SearchKeywordInput;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::Mutex;
use vitis_be_macros::macroql;

macroql! {
    pub query search (
        searchKeywordInput: SearchKeywordInput {
            keyword: String
        }
    ) {
        searchKeyword(searchKeywordInput) {
            list: [] {
                ...NormalListViewItem {
                    thumbnail: String,
                    row1: String,
                    row2: [String],
                    scheme: String
                }
            },
            isEnd: Boolean,
        }
    }
}
#[tokio::main]
async fn main() {
    let client = Client::new();
    let res = client
        .search(search::Vars {
            search_keyword_input: SearchKeywordInput {
                keyword: "나 혼자".to_string(),
            },
        })
        .await
        .unwrap();
    println!("{:#?}", res)
}

struct Client {
    inner: reqwest::Client,
    token: Arc<Mutex<Option<String>>>,
}

#[derive(Serialize)]
struct Req<'a, T> {
    query: &'a str,
    variables: T,
}

#[derive(Deserialize)]
struct Success<T> {
    data: T,
}

#[derive(Deserialize)]
struct Failure {
    errors: Vec<Error>,
}

#[derive(Deserialize)]
struct Error {
    message: String,
}

impl Client {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
            token: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn req<'a, V: Serialize, S: DeserializeOwned>(
        &self,
        query: &'a str,
        variables: V,
    ) -> Result<S> {
        let mut headers = HeaderMap::new();
        if let Some(token) = self.token.lock().await.as_ref() {
            headers.insert("cookie", format!("_kpwtkn={token}").parse()?);
        }
        headers.insert("user-agent", "kakaopage".parse()?);
        headers.insert("referer", "https://page.kakao.com".parse()?);
        let res = self
            .inner
            .post("https://page.kakao.com/graphql")
            .headers(headers)
            .json(&Req { query, variables })
            .send()
            .await?;
        if res.status().is_success() {
            Ok(res.json::<Success<S>>().await?.data)
        } else {
            let errors = res
                .json::<Failure>()
                .await?
                .errors
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join(", ");
            Err(anyhow!("{errors}"))
        }
    }
}

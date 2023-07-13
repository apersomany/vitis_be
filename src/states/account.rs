use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use anyhow::Result;
use axum::http::HeaderMap;
use cookie::Cookie;
use rand::random;
use reqwest::{cookie::CookieStore, header::HeaderValue, Client, Proxy, Url};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use vitis_be_macros::macroql;

use crate::util::{get_param, iso};

use self::{
    draw_gotcha::vars::DrawGotchaInput, gotchas::vars::MyNewsListInput,
    recv_ticket::vars::TicketFreeMutationInput,
};

use super::States;

#[derive(Serialize, Deserialize)]
pub struct Account {
    #[serde(default)]
    pub last_token_refresh: i64,
    #[serde(default)]
    pub last_gotcha_opened: i64,
    #[serde(default)]
    pub balance: i64,
    token: Arc<Token>,
    #[serde(default = "generate_agent")]
    agent: String,
    proxy: Option<String>,
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
    format!("kakaopage/{:016x}", random::<u64>())
}

macroql! {
    query balance {
        userAndCash {
            cash {
                remainCash: Long
            }
        },
    }
}

macroql! {
    query gotchas (
        myNewsListInput: MyNewsListInput {
            tab: String,
            refresh: Boolean
        }
    ) {
        myNewsList(myNewsListInput) {
            news: [] {
                logName: String,
                date: String,
                scheme: String
            }
        }
    }
}

macroql! {
    mutation draw_gotcha (
        input: DrawGotchaInput {
            gotchaId: String
        }
    ) {
        drawGotcha(input) {
            status: String
        }
    }
}

macroql! {
    query tickets {
        todayGiftList {
            list: [] {
                isReceived: Boolean,
                ticketUid: Int,
                scheme: String
            }
        }
    }
}

macroql! {
    mutation recv_ticket (
        input: TicketFreeMutationInput {
            ticketUid: Int,
            typ_: String
        }
    ) {
        receiveTicketFree(input) {
            isReceived: Boolean,
            ticketCount: Long
        }
    }
}

impl Account {
    pub fn client(&self) -> Client {
        self.client
            .get_or_init(|| {
                let mut headers = HeaderMap::new();
                headers.insert("referer", "https://page.kakao.com".parse().unwrap());
                let mut builder = Client::builder()
                    .cookie_provider(self.token.clone())
                    .default_headers(headers)
                    .user_agent(&self.agent);
                if let Some(proxy) = &self.proxy {
                    builder = builder.proxy(Proxy::http(proxy).unwrap())
                }
                builder.build().unwrap()
            })
            .clone()
    }

    pub async fn refresh_token(states: &States, key: i64) -> Result<()> {
        states
            .get_acc(key)?
            .client()
            .head("https://page.kakao.com")
            .send()
            .await?;
        Ok(())
    }

    pub async fn check_balance(states: &States, key: i64) -> Result<()> {
        let sels = balance(states.get_acc(key)?.client(), balance::Vars {}).await?;
        states.get_acc(key)?.balance = sels.user_and_cash.cash.remain_cash;
        Ok(())
    }

    pub async fn check_gotchas(states: &States, key: i64) -> Result<()> {
        let sels = gotchas(
            states.get_acc(key)?.client(),
            gotchas::Vars {
                my_news_list_input: MyNewsListInput {
                    tab: "ALL".to_string(),
                    refresh: true,
                },
            },
        )
        .await?;
        let mut max_date = 0;
        for news in sels.my_news_list.news {
            if news.log_name == "Award" {
                let date = iso(&news.date)?;
                if date > states.get_acc(key)?.last_gotcha_opened {
                    let gotcha_id = if let Ok(scheme) = get_param(
                        &urlencoding::decode(&news.scheme)?,
                        "open_enc_url_with_auth",
                    ) {
                        get_param(&urlencoding::decode(&scheme)?, "id")?
                    } else {
                        get_param(&urlencoding::decode(&news.scheme)?, "gacha_uid")?
                    };
                    draw_gotcha(
                        states.get_acc(key)?.client(),
                        draw_gotcha::Vars {
                            input: DrawGotchaInput { gotcha_id },
                        },
                    )
                    .await?;
                    max_date = max_date.max(date);
                    sleep(Duration::from_millis(200 + random::<u64>() % 200)).await;
                }
            }
        }
        if max_date > 0 {
            states.get_acc(key)?.last_gotcha_opened = max_date;
        }
        Ok(())
    }

    pub async fn check_tickets(states: &States, key: i64) -> Result<()> {
        let sels = tickets(states.get_acc(key)?.client(), tickets::Vars {}).await?;
        for gift in sels.today_gift_list.list {
            if !gift.is_received {
                let _ = recv_ticket(
                    states.get_acc(key)?.client(),
                    recv_ticket::Vars {
                        input: TicketFreeMutationInput {
                            typ_: "TodayGift".to_string(),
                            ticket_uid: gift.ticket_uid,
                        },
                    },
                )
                .await?;
                // disabled as it would complicate the check first time free ticket logic
                // states
                //     .get_srs(get_param(&gift.scheme, "series_id")?.parse()?)?
                //     .get_tkt(key)?
                //     .permanent
                //     .add_assign(sels.receive_ticket_free.ticket_count);
                sleep(Duration::from_millis(200 + random::<u64>() % 200)).await;
            }
        }
        Ok(())
    }
}

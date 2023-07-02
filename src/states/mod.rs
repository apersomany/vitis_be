use std::{fs::File, sync::Arc};

use anyhow::Result;
use axum::http::{HeaderMap, HeaderValue};
use dashmap::DashMap;
use log::{info, warn};
use reqwest::Client;
use tokio::task::spawn_blocking;

use self::{
    account::{generate_agent, Account},
    config::Config,
    series::Series,
};

pub mod account;
pub mod config;
pub mod series;

pub struct States {
    pub accounts: DashMap<i32, Account>,
    pub serieses: DashMap<i32, Series>,
    pub config: Config,
    pub client: Client,
}

impl States {
    pub async fn load() -> Result<Arc<Self>> {
        spawn_blocking(move || {
            Ok(Arc::new(Self {
                accounts: if let Ok(reader) = File::open("accounts.json") {
                    info!("loading accounts");
                    serde_json::from_reader(reader)?
                } else {
                    warn!("accounts.json not found, using default value");
                    Default::default()
                },
                serieses: if let Ok(reader) = File::open("serieses.json") {
                    info!("loading serieses");
                    serde_json::from_reader(reader)?
                } else {
                    warn!("serieses.json not found, using default value");
                    Default::default()
                },
                config: if let Ok(reader) = File::open("accounts.json") {
                    info!("loading config");
                    serde_json::from_reader(reader)?
                } else {
                    warn!("config.json not found, using default value");
                    Default::default()
                },
                client: {
                    let mut headers = HeaderMap::new();
                    headers.insert("referer", "https://page.kakao.com".parse()?);
                    Client::builder()
                        .user_agent(generate_agent())
                        .default_headers(headers)
                        .build()?
                },
            }))
        })
        .await?
    }

    pub async fn save(self: Arc<Self>) -> Result<()> {
        spawn_blocking(move || {
            info!("saving accounts");
            let accounts_writer = File::create("accounts.json")?;
            serde_json::to_writer_pretty(accounts_writer, &self.accounts)?;
            info!("saving serieses");
            let serieses_writer = File::create("serieses.json")?;
            serde_json::to_writer_pretty(serieses_writer, &self.serieses)?;
            info!("saving timers");
            let config_writer = File::create("config.json")?;
            serde_json::to_writer_pretty(config_writer, &self.config)?;
            Ok(())
        })
        .await?
    }
}

use std::{fs::File, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::http::HeaderMap;
use dashmap::{mapref::one::RefMut, DashMap};
use log::{info, warn};
use rand::random;
use reqwest::Client;
use tokio::{spawn, time::sleep};

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
    pub fn get_account(&self, key: i32) -> Result<RefMut<i32, Account>> {
        self.accounts
            .get_mut(&key)
            .with_context(move || format!("account {key} does not exist"))
    }

    pub fn get_series(&self, key: i32) -> Result<RefMut<i32, Series>> {
        if let Some(series) = self.serieses.get_mut(&key) {
            Ok(series)
        } else {
            self.serieses.insert(key, Series::default());
            Ok(self.serieses.get_mut(&key).unwrap())
        }
    }

    pub fn load() -> Result<Arc<Self>> {
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
            config: if let Ok(reader) = File::open("config.json") {
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
    }

    pub fn save(&self) -> Result<()> {
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
    }

    pub fn start_timers(self: &Arc<Self>) {
        for key in self.accounts.iter().map(|e| *e.key()) {
            let states = self.clone();
            spawn(async move {
                loop {
                    if let Err(e) = states.accounts.get(&key).unwrap().refresh_token().await {
                        warn!("failed to refresh token for account {key}: {e}")
                    }
                    sleep(Duration::from_secs(2400 + random::<u64>() % 2400)).await;
                }
            });
            let states = self.clone();
            spawn(async move {
                loop {
                    if let Err(e) = Account::check_gotchas(&states, key).await {
                        warn!("failed to check gotchas for account {key}: {e}")
                    }
                    if let Err(e) = Account::check_balance(&states, key).await {
                        warn!("failed to check balance for account {key}: {e}")
                    }
                    sleep(Duration::from_secs(2400 + random::<u64>() % 2400)).await;
                }
            });
            let states = self.clone();
            spawn(async move {
                loop {
                    if let Err(e) = Account::check_tickets(&states, key).await {
                        warn!("failed to check tickets for account {key}: {e}")
                    }
                    sleep(Duration::from_secs(9600 + random::<u64>() % 9600)).await;
                }
            });
            break;
        }
    }
}

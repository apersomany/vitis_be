use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use axum::http::HeaderMap;
use dashmap::{mapref::one::RefMut, DashMap};
use log::{info, warn};
use rand::random;
use reqwest::Client;
use tokio::{spawn, sync::broadcast::Receiver, time::sleep};

use crate::util::{now, spawn_solo};

use self::{
    account::{generate_agent, Account},
    config::Config,
    series::Series,
};

pub mod account;
pub mod config;
pub mod series;

pub struct States {
    pub accounts: DashMap<i64, Account>,
    pub serieses: DashMap<i64, Series>,
    pub find_map: DashMap<i64, Receiver<bool>>,
    pub config: Config,
    pub client: Client,
}

impl States {
    pub fn get_acc(&self, key: i64) -> Result<RefMut<i64, Account>> {
        self.accounts
            .get_mut(&key)
            .with_context(move || format!("account {key} does not exist"))
    }

    pub fn get_srs(&self, key: i64) -> Result<RefMut<i64, Series>> {
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
                let reader = BufReader::new(reader);
                serde_json::from_reader(reader)?
            } else {
                warn!("accounts.json not found, using default value");
                Default::default()
            },
            serieses: if let Ok(reader) = File::open("serieses.json") {
                info!("loading serieses");
                let reader = BufReader::new(reader);
                serde_json::from_reader(reader)?
            } else {
                warn!("serieses.json not found, using default value");
                Default::default()
            },
            find_map: { DashMap::new() },
            config: if let Ok(reader) = File::open("config.json") {
                info!("loading config");
                let reader = BufReader::new(reader);
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

    // todo: abstract the remove/rename/rename
    pub fn save(&self) -> Result<()> {
        info!("saving accounts");
        let accounts_writer = File::create("accounts.json.new")?;
        let accounts_writer = BufWriter::new(accounts_writer);
        serde_json::to_writer_pretty(accounts_writer, &self.accounts)?;
        let _ = fs::remove_file("accounts.json.old");
        let _ = fs::rename("accounts.json", "accounts.json.old");
        let _ = fs::rename("accounts.json.new", "accounts.json");
        info!("saving serieses");
        let serieses_writer = File::create("serieses.json.new")?;
        let serieses_writer = BufWriter::new(serieses_writer);
        serde_json::to_writer_pretty(serieses_writer, &self.serieses)?;
        let _ = fs::remove_file("serieses.json.old");
        let _ = fs::rename("serieses.json", "serieses.json.old");
        let _ = fs::rename("serieses.json.new", "serieses.json");
        info!("saving config");
        let config_writer = File::create("config.json.new")?;
        let config_writer = BufWriter::new(config_writer);
        serde_json::to_writer_pretty(config_writer, &self.config)?;
        let _ = fs::remove_file("config.json.old");
        let _ = fs::rename("config.json", "config.json.old");
        let _ = fs::rename("config.json.new", "config.json");
        Ok(())
    }

    pub fn start_timers(self: &Arc<Self>) {
        for key in self.accounts.iter().map(|e| *e.key()) {
            let states = self.clone();
            spawn(async move {
                loop {
                    let states = states.clone();
                    let diff = now() - states.get_acc(key).unwrap().last_token_refresh;
                    if diff < 3600 {
                        sleep(Duration::from_secs(3600 - diff as u64)).await;
                    }
                    let _ = spawn_solo(async move {
                        if let Err(e) = Account::refresh_token(&states, key).await {
                            warn!("failed to refresh token for account {key}: {e}")
                        } else {
                            info!("refreshed token for account {key}")
                        }
                        states.get_acc(key).unwrap().last_token_refresh = now();
                    })
                    .await;
                }
            });
            let states = self.clone();
            spawn(async move {
                loop {
                    sleep(Duration::from_secs(2400 + random::<u64>() % 2400)).await;
                    let states = states.clone();
                    spawn_solo(async move {
                        if let Err(e) = Account::check_gotchas(&states, key).await {
                            warn!("failed to check gotchas for account {key}: {e}")
                        }
                        if let Err(e) = Account::check_balance(&states, key).await {
                            warn!("failed to check balance for account {key}: {e}")
                        }
                    });
                }
            });
            let states = self.clone();
            spawn(async move {
                loop {
                    sleep(Duration::from_secs(9600 + random::<u64>() % 9600)).await;
                    let states = states.clone();
                    spawn_solo(async move {
                        if let Err(e) = Account::check_tickets(&states, key).await {
                            warn!("failed to check tickets for account {key}: {e}")
                        }
                    });
                }
            });
        }
        let states = self.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(3600));
            states.clone().save().unwrap();
        });
    }
}

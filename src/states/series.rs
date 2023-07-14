use anyhow::{Context, Result};
use dashmap::{mapref::one::RefMut, DashMap};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Series {
    pub single_map: DashMap<i64, Single>,
    pub ticket_map: DashMap<i64, Ticket>,
}

impl Series {
    pub fn get_tkt(&self, key: i64) -> Result<RefMut<i64, Ticket>> {
        self.ticket_map
            .get_mut(&key)
            .with_context(move || format!("account {key} does not exist for this series"))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Single {
    pub title: String,
    pub viewer: Viewer,
    pub prev: Option<i64>,
    pub next: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum Viewer {
    ImageList(Vec<Image>),
    KakaoHTML(Vec<KHTML>),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Image {
    pub size: i64,
    pub kid: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct KHTML {
    pub chapter_id: i64,
    pub content_id: i64,
    pub kid: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Ticket {
    pub wait_free: i64,
    pub permanent: i64,
}

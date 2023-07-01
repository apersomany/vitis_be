use chrono::NaiveDateTime;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Series {
    pub single_map: DashMap<i32, Single>,
    pub ticket_map: DashMap<i32, Ticket>,
}

#[derive(Serialize, Deserialize)]
pub enum Single {
    KakaoHTML(Vec<Vec<String>>),
    ImageList(Vec<String>),
}

#[derive(Serialize, Deserialize)]
pub struct Ticket {
    pub wait_free: NaiveDateTime,
    pub permanent: i32,
}

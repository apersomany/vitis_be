use anyhow::Result;
use chrono::NaiveDateTime;
use dashmap::{mapref::one::RefMut, DashMap};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Series {
    pub single_map: DashMap<i32, Single>,
    pub ticket_map: DashMap<i32, Ticket>,
}

impl Series {
    pub fn get_ticket(&self, key: i32) -> Result<RefMut<i32, Ticket>> {
        if let Some(ticket) = self.ticket_map.get_mut(&key) {
            Ok(ticket)
        } else {
            self.ticket_map.insert(key, Ticket::default());
            Ok(self.ticket_map.get_mut(&key).unwrap())
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Single {
    KakaoHTML(Vec<Vec<String>>),
    ImageList(Vec<String>),
}

#[derive(Default, Serialize, Deserialize)]
pub struct Ticket {
    pub wait_free: NaiveDateTime,
    pub permanent: i32,
}

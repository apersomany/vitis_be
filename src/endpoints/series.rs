use serde::Deserialize;

#[derive(Deserialize)]
pub struct SeriesReq {
    series_id: i32,
    page: Option<i32>,
}

pub struct SeriesRes {}

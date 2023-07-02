use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use vitis_be_macros::macroql;

use crate::states::States;

use self::search_keyword::{sels::search_keyword::List, vars::SearchKeywordInput, Vars};

use super::{get_param, Result};

#[derive(Deserialize)]
pub struct SearchReq {
    keyword: String,
    page: Option<i32>,
}

#[derive(Serialize)]
pub struct SearchRes {
    data: Vec<Series>,
    more: bool,
}

#[derive(Serialize)]
pub struct Series {
    series_id: i32,
    cover: String,
    title: String,
    row_1: String,
    row_2: String,
}

macroql! {
    query searchKeyword (
        searchKeywordInput: SearchKeywordInput {
            keyword: String,
            page: Int
        }
    ) {
        searchKeyword(searchKeywordInput) {
            list: [] {
                ... NormalListViewItem {
                    thumbnail: String,
                    row1: String,
                    row2: [String],
                    row3: {
                        metaList: [String]
                    },
                    scheme: String
                }
            },
            isEnd: Boolean
        }
    }
}

pub async fn search(
    State(state): State<Arc<States>>,
    Query(query): Query<SearchReq>,
) -> Result<Json<SearchRes>> {
    let sels: search_keyword::Sels = search_keyword(
        &state.client,
        Vars {
            search_keyword_input: SearchKeywordInput {
                keyword: query.keyword,
                page: query.page.unwrap_or(0),
            },
        },
    )
    .await?;
    let mut data = Vec::new();
    for item in sels.search_keyword.list {
        match item {
            List::NormalListViewItem {
                thumbnail,
                row_1,
                row_2,
                row_3,
                scheme,
            } => data.push(Series {
                series_id: get_param(&scheme, "series_id")?.parse()?,
                cover: get_param(&thumbnail, "kid")?,
                title: row_1,
                row_1: row_2.join("·"),
                row_2: row_3.meta_list.join("·"),
            }),
            List::Unknown => {}
        }
    }
    let more = !sels.search_keyword.is_end;
    Ok(Json(SearchRes { data, more }))
}

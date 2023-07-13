use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use vitis_be_macros::macroql;

use crate::{states::States, util::get_param};

use self::search_keyword::{vars::SearchKeywordInput, Vars};

use super::Result;

#[derive(Deserialize)]
pub struct SearchReq {
    keyword: String,
    #[serde(default)]
    page: i32,
}

#[derive(Serialize)]
pub struct SearchRes {
    list: Vec<Series>,
    more: bool,
}

#[derive(Serialize)]
pub struct Series {
    series_id: i64,
    cover: String,
    title: String,
    row_1: String,
    row_2: String,
}

macroql! {
    query search_keyword (
        searchKeywordInput: SearchKeywordInput {
            keyword: String,
            page: Int
        }
    ) {
        searchKeyword(searchKeywordInput) {
            list: [] {
                    thumbnail: String,
                    row1: String,
                    row2: [String],
                    row3: {
                        metaList: [String]
                    },
                    scheme: String
            },
            isEnd: Boolean
        }
    }
}

pub async fn search(
    State(state): State<Arc<States>>,
    Query(query): Query<SearchReq>,
) -> Result<Json<SearchRes>> {
    let sels = search_keyword(
        state.client.clone(),
        Vars {
            search_keyword_input: SearchKeywordInput {
                keyword: query.keyword,
                page: query.page,
            },
        },
    )
    .await?;
    let mut list = Vec::new();
    for item in sels.search_keyword.list {
        list.push(Series {
            series_id: get_param(&item.scheme, "series_id")?.parse()?,
            cover: get_param(&item.thumbnail, "kid")?,
            title: item.row_1,
            row_1: item.row_2.join(" · "),
            row_2: item.row_3.meta_list.join(" · "),
        });
    }
    let more = !sels.search_keyword.is_end;
    Ok(Json(SearchRes { list, more }))
}

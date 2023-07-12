use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use vitis_be_macros::macroql;

use crate::{states::States, util::get_param};

use super::Result;

#[derive(Deserialize)]
pub struct SeriesReq {
    series_id: i64,
    #[serde(default)]
    page: i32,
    #[serde(default)]
    sort: Sort,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Sort {
    #[default]
    Dsc,
    Asc,
}

impl ToString for Sort {
    fn to_string(&self) -> String {
        match &self {
            Sort::Dsc => "",
            Sort::Asc => "asc",
        }
        .to_string()
    }
}

#[derive(Serialize)]
pub struct SeriesRes {
    meta: Option<Series>,
    list: Vec<Single>,
    more: bool,
}

#[derive(Serialize)]
struct Series {
    cover: String,
    title: String,
    pub_period: Option<String>,
    view_count: i64,
    rating: f64,
    author: String,
    description: String,
}

#[derive(Serialize)]
struct Single {
    single_id: i64,
    cover: String,
    title: String,
    row_1: String,
    row_2: Option<String>,
}

macroql! {
    query series_full (
        sortType: String,
        seriesId: Long,
    ) {
        contentHomeOverview(seriesId) {
            content: {
                thumbnail: String,
                title: String,
                authors: String,
                pubPeriod: String?,
                serviceProperty: {
                    viewCount: Long,
                    ratingCount: Long,
                    ratingSum: Long
                }
            }
        },
        contentHomeAbout(seriesId) {
            description: String,
        },
        contentHomeProductList(sortType, seriesId) {
            pageInfo {
                hasNextPage: Boolean
            },
            edges: [] {
                node: {
                    thumbnail: String,
                    row1: {
                        title: String
                    },
                    row2: [String],
                    row3: String?,
                    scheme: String
                }
            }
        }
    }
}

macroql! {
    query single_list (
        sortType: String,
        seriesId: Long,
        after: String,
    ) {
        contentHomeProductList(sortType, seriesId, after) {
            pageInfo {
                hasNextPage: Boolean
            },
            edges: [] {
                node: {
                    thumbnail: String,
                    row1: {
                        title: String
                    },
                    row2: [String],
                    row3: String?,
                    scheme: String
                }
            }
        }
    }
}

pub async fn series(
    State(state): State<Arc<States>>,
    Query(query): Query<SeriesReq>,
) -> Result<Json<SeriesRes>> {
    macro_rules! list {
        ($sels:ident) => {{
            let mut list = Vec::new();
            for item in $sels.content_home_product_list.edges {
                list.push(Single {
                    single_id: get_param(&item.node.scheme, "product_id")?.parse()?,
                    cover: get_param(&item.node.thumbnail, "kid")?.parse()?,
                    title: item.node.row_1.title,
                    row_1: item.node.row_2.join(" · "),
                    row_2: item.node.row_3,
                })
            }
            list
        }};
    }
    if query.page == 0 {
        let sels = series_full(
            &state.client,
            series_full::Vars {
                sort_type: query.sort.to_string(),
                series_id: query.series_id,
            },
        )
        .await?;
        Ok(Json(SeriesRes {
            meta: Some(Series {
                cover: get_param(&sels.content_home_overview.content.thumbnail, "kid")?,
                title: sels.content_home_overview.content.title,
                pub_period: sels.content_home_overview.content.pub_period,
                view_count: sels
                    .content_home_overview
                    .content
                    .service_property
                    .view_count,
                rating: {
                    let num = sels
                        .content_home_overview
                        .content
                        .service_property
                        .rating_sum;
                    let den = sels
                        .content_home_overview
                        .content
                        .service_property
                        .rating_count;
                    num as f64 / den as f64
                },
                author: sels.content_home_overview.content.authors,
                description: sels.content_home_about.description,
            }),
            list: list!(sels),
            more: sels.content_home_product_list.page_info.has_next_page,
        }))
    } else {
        let sels = single_list(
            &state.client,
            single_list::Vars {
                sort_type: query.sort.to_string(),
                series_id: query.series_id,
                after: (query.page * 25).to_string(),
            },
        )
        .await?;
        Ok(Json(SeriesRes {
            meta: None,
            list: list!(sels),
            more: sels.content_home_product_list.page_info.has_next_page,
        }))
    }
}

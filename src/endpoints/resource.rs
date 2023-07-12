use std::sync::Arc;

use axum::{
    extract::{OriginalUri, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};

use crate::states::States;

use super::Result;

pub async fn resource(State(state): State<Arc<States>>, oguri: OriginalUri) -> Result<Response> {
    let url = "https://dn-img-page.kakao.com".to_string() + oguri.to_string().as_str();
    let res = state.client.get(url).send().await?;
    let res = (
        res.status(),
        res.headers()
            .into_iter()
            .filter_map(|(key, val)| match key.as_str() {
                "content-type" => Some((key.clone(), val.clone())),
                _ => None,
            })
            .collect::<HeaderMap>(),
        res.bytes().await?,
    );
    Ok(res.into_response())
}

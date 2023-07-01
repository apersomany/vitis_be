use anyhow::Result;
use axum::{routing::get, Router, Server};
use endpoints::search::search;
use env_logger::{Env, DEFAULT_FILTER_ENV};
use states::States;

pub mod endpoints;
pub mod states;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(Env::new().filter_or(DEFAULT_FILTER_ENV, "info"));
    let states = States::load().await?;
    let router = Router::new()
        .route("/search", get(search))
        .with_state::<()>(states.clone());
    Server::bind(&states.config.bind_addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}

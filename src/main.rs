use anyhow::Result;
use axum::{
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router, Server,
};
use endpoints::{search::search, series::series};
use env_logger::{Env, DEFAULT_FILTER_ENV};
use states::States;

pub mod endpoints;
pub mod states;

async fn cors<T>(request: Request<T>, next: Next<T>) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("access-control-allow-origin", "*".parse().unwrap());
    response
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(Env::new().filter_or(DEFAULT_FILTER_ENV, "info"));
    let states = States::load().await?;
    let router = Router::new()
        .route("/search", get(search))
        .route("/series", get(series))
        .layer(middleware::from_fn(cors))
        .with_state::<()>(states.clone());
    Server::bind(&states.config.bind_addr)
        .serve(router.into_make_service())
        .await?;
    Ok(())
}

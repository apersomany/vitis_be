use anyhow::Result;
use axum::{
    body::Body,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router, Server,
};
use endpoints::{resource::resource, search::search, series::series, single::single};
use env_logger::{Env, DEFAULT_FILTER_ENV};
use log::info;
use states::States;

pub mod endpoints;
pub mod states;
pub mod util;

async fn cors(request: Request<Body>, next: Next<Body>) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("access-control-allow-origin", "*".parse().unwrap());
    response
}

async fn tsig() {
    tokio::signal::ctrl_c().await.unwrap();
    info!("stopping")
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(Env::new().filter_or(DEFAULT_FILTER_ENV, "info"));
    let states = States::load()?;
    let router = Router::new()
        .route("/:resty/resource", get(resource))
        .route("/search", get(search))
        .route("/series", get(series))
        .route("/single", get(single))
        .layer(middleware::from_fn(cors))
        .with_state::<()>(states.clone());
    states.start_timers();
    Server::bind(&states.config.bind_addr)
        .serve(router.into_make_service())
        .with_graceful_shutdown(tsig())
        .await?;
    states.save()?;
    Ok(())
}

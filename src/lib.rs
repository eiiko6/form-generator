use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    Router,
    http::{Method, header},
    routing::{get, post},
};
use tokio::sync::Mutex;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};

pub mod handlers;
pub use handlers::{AppState, render_form, submit};

pub mod config;
pub use config::load_config;

use crate::handlers::AppConfig;

/// Start the server with the given configuration and output file.
/// `addr` should be something like "127.0.0.1:8081"
pub async fn run_server(cfg: AppConfig, output_file: String, addr: &str) -> anyhow::Result<()> {
    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    // rate limiter
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(3)
        .burst_size(10)
        .finish()
        .unwrap();

    // a separate background task to clean up
    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(interval);
            // tracing::info!("rate limiting storage size: {}", governor_limiter.len());
            governor_limiter.retain_recent();
        }
    });

    let state = AppState {
        cfg: Arc::new(cfg),
        file_lock: Arc::new(Mutex::new(())),
        output_file,
    };

    let app = Router::new()
        .route("/", get(render_form))
        .route("/submit", post(submit))
        .with_state(state)
        .layer(cors)
        .layer(GovernorLayer::new(governor_conf));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening on {}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

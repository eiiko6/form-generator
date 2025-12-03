use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    Router,
    http::{Method, header},
    routing::{get, post},
};
use tokio::sync::Mutex;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};

use clap::Parser;

mod handlers;
use handlers::{AppState, render_form, submit};
mod config;
use config::load_config;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Config path
    #[arg(short, long, default_value = "config.toml")]
    config_path: String,

    /// Output file
    #[arg(short, long, default_value = "answers.json")]
    output_file: String,

    /// Verbose mode
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).ok();

    // load config
    let cfg = load_config(&cli.config_path).context(format!("loading {}", cli.config_path))?;
    tracing::info!(
        "Loaded config: '{}', writing answers to '{}' with {} fields",
        cli.config_path,
        cli.output_file,
        cfg.fields.len()
    );

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
        output_file: cli.output_file.clone(),
    };

    let app = Router::new()
        .route("/", get(render_form))
        .route("/submit", post(submit))
        .with_state(state)
        .layer(cors)
        .layer(GovernorLayer::new(governor_conf));

    let port = std::env::var("SERVER_PORT").unwrap_or("8081".to_string());
    let addr = format!("127.0.0.1:{port}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on {addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}

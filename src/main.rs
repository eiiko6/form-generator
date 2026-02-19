use std::{net::SocketAddr, time::Duration};

use anyhow::Context;
use axum::{
    Router,
    http::{Method, header},
    response::Redirect,
    routing::get,
};
use clap::Parser;

use form_generator::config::load_config;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config_path: String,

    #[arg(short, long)]
    output_file: Option<String>,

    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).ok();

    let cfg = load_config(&cli.config_path).context(format!("loading {}", cli.config_path))?;

    let output_file = cli
        .output_file
        .or_else(|| cfg.json_output.clone())
        .unwrap_or_else(|| "answers.json".to_string());

    tracing::info!(
        "Loaded config: '{}', writing answers to '{}' with {} fields",
        cli.config_path,
        output_file,
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
    std::thread::spawn(move || {
        let interval = Duration::from_secs(60);
        loop {
            std::thread::sleep(interval);
            governor_limiter.retain_recent();
        }
    });

    let app = Router::new()
        .merge(form_generator::app_router(
            cfg,
            output_file,
            "/form",
            "/submit",
        ))
        .route("/", get(form_redirect))
        .layer(cors)
        .layer(GovernorLayer::new(governor_conf));

    let port = std::env::var("SERVER_PORT").unwrap_or("8081".to_string());
    let addr = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn form_redirect() -> Redirect {
    Redirect::to("/form")
}

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use askama::Template;
use axum::{
    Router,
    extract::{Form, State},
    http::{Method, StatusCode, header},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Deserialize)]
struct FieldDef {
    name: String,
    title: String,
    description: String,
    answer_type: String,
    html_before: Option<String>,
    html_after: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    json_output: String,
    submit_button: String,
    fields: Vec<FieldDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResponseEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    answers: HashMap<String, String>,
}

#[derive(Clone)]
struct AppState {
    cfg: Arc<AppConfig>,
    file_lock: Arc<Mutex<()>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).ok();

    // load config
    let cfg = load_config("config.toml").context("loading config.toml")?;
    tracing::info!(
        "Loaded config: json_output='{}' with {} fields",
        cfg.json_output,
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
    };

    let app = Router::new()
        .route("/", get(render_form))
        .route("/submit", post(submit))
        .with_state(state)
        .layer(cors)
        .layer(GovernorLayer::new(governor_conf));

    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8081".to_string());
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

async fn render_form(State(state): State<AppState>) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "form.html")]
    struct FormTemplate<'a> {
        fields: &'a [FieldDef],
        lang: &'a str,
        submit_button: &'a str,
    }

    let tmpl = FormTemplate {
        fields: &state.cfg.fields,
        lang: "en",
        submit_button: &state.cfg.submit_button,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template render error").into_response(),
    }
}

async fn submit(
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let entry = ResponseEntry {
        timestamp: Utc::now(),
        answers: form,
    };

    let _guard = state.file_lock.lock().await;
    let path = &state.cfg.json_output;

    let mut existing: Vec<ResponseEntry> = match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| Vec::new()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            tracing::error!("Failed to read {}: {}", path, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read storage file",
            )
                .into_response();
        }
    };

    existing.push(entry.clone());

    if let Err(e) = std::fs::write(path, serde_json::to_string_pretty(&existing).unwrap()) {
        tracing::error!("Failed to write {}: {}", path, e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to write storage file",
        )
            .into_response();
    }

    // tracing::info!("Entry submitted: {:?}", entry.answers);

    Html(
        r#"
        <html>
            <body>
                <p>Saved. <a href="/">Back</a></p>
            </body>
        </html>
        "#,
    )
    .into_response()
}

/// Load config from path
fn load_config(path: &str) -> anyhow::Result<AppConfig> {
    let raw = std::fs::read_to_string(path)?;
    let cfg: AppConfig = toml::from_str(&raw)?;

    // field names must be unique and non-empty
    let mut seen = std::collections::HashSet::new();
    for f in &cfg.fields {
        if f.name.trim().is_empty() {
            anyhow::bail!("field with empty name in config");
        }
        if !seen.insert(f.name.clone()) {
            anyhow::bail!("duplicate field name in config: {}", f.name);
        }
    }

    Ok(cfg)
}

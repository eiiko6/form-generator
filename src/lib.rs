use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use tokio::sync::Mutex;

pub mod handlers;
pub use handlers::{AppState, render_form, submit};

pub mod config;
pub use config::load_config;

use crate::handlers::AppConfig;

/// Returns an `axum::Router` configured with the `/form` and `/submit` routes
/// This router can be merged into another router using `merge`.
pub fn app_router(
    cfg: AppConfig,
    output_file: String,
    form_route: &str,
    submit_route: &str,
) -> Router {
    let state = AppState {
        cfg: Arc::new(cfg),
        file_lock: Arc::new(Mutex::new(())),
        output_file,
    };

    Router::new()
        .route(form_route, get(render_form))
        .route(submit_route, post(submit))
        .with_state(state)
}

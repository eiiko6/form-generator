use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub title: String,
    pub description: String,
    pub answer_type: String,
    pub html_before: Option<String>,
    pub html_after: Option<String>,
    pub options: Option<Vec<String>>,
}

pub enum FieldWidget<'a> {
    Checkbox,
    Textarea,
    Select(&'a [String]),
    Input(&'a str),
}

impl FieldDef {
    pub fn widget(&self) -> FieldWidget<'_> {
        match self.answer_type.as_str() {
            "checkbox" => FieldWidget::Checkbox,
            "textarea" => FieldWidget::Textarea,
            "select" => FieldWidget::Select(self.options.as_deref().unwrap_or(&[])),
            other => FieldWidget::Input(other),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResponseEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    answers: HashMap<String, Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub json_output: Option<String>,
    pub form_title: String,
    pub submit_button: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<AppConfig>,
    pub file_lock: Arc<Mutex<()>>,
    pub output_file: String,
}

pub async fn render_form(State(state): State<AppState>) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "form.html")]
    struct FormTemplate<'a> {
        form_title: &'a str,
        submit_button: &'a str,
        fields: &'a [FieldDef],
        lang: &'a str,
    }

    let tmpl = FormTemplate {
        form_title: &state.cfg.form_title,
        submit_button: &state.cfg.submit_button,
        fields: &state.cfg.fields,
        lang: "en",
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template render error").into_response(),
    }
}

pub async fn submit(
    State(state): State<AppState>,
    Form(mut form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut processed_answers = HashMap::new();

    for field in &state.cfg.fields {
        let value = form
            .remove(&field.name)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        processed_answers.insert(field.name.clone(), value);
    }

    for (key, val) in form {
        let value = Some(val.trim().to_string()).filter(|s| !s.is_empty());
        processed_answers.insert(key, value);
    }

    let entry = ResponseEntry {
        timestamp: Utc::now(),
        answers: processed_answers,
    };

    let _guard = state.file_lock.lock().await;
    let path = &state.output_file;

    let mut existing: Vec<ResponseEntry> = match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(Vec::new()),
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

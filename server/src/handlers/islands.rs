use crate::models::AppState;
use axum::{
    Form, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use tracing;

pub async fn list(State(_state): State<AppState>) -> impl IntoResponse {
    super::serve_html("islands").await
}

pub async fn new_form(State(_state): State<AppState>) -> impl IntoResponse {
    super::serve_html("islands-new").await
}

pub async fn create(
    State(state): State<AppState>,
    Form(params): Form<serde_json::Value>,
) -> impl IntoResponse {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("New Island")
        .to_string();

    let topic = params
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("general")
        .to_string();

    let description = params
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    tracing::info!("Creating vocabulary island: {} (topic: {})", name, topic);

    // In production, save to database
    Json(json!({
        "success": true,
        "name": name,
        "topic": topic,
        "description": description
    }))
}

pub async fn view(Path(id): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, format!("Vocabulary Island: {}", id))
}

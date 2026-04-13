use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Form,
    Json,
};
use serde_json::json;
use tracing;
use crate::models::AppState;

pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, "Your Vocabulary Islands")
}

pub async fn new_form() -> impl IntoResponse {
    (StatusCode::OK, "Create New Vocabulary Island")
}

pub async fn create(
    State(state): State<AppState>,
    Form(params): Form<serde_json::Value>,
) -> impl IntoResponse {
    let name = params.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("New Island")
        .to_string();
    
    let topic = params.get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("general")
        .to_string();
    
    let description = params.get("description")
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

pub async fn view(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    (StatusCode::OK, format!("Vocabulary Island: {}", id))
}

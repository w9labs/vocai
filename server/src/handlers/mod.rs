pub mod auth;
pub mod flashcards;
pub mod islands;
pub mod review;

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    Json,
};
use tokio::fs;
use crate::models::AppState;

pub async fn index() -> impl IntoResponse {
    match fs::read_to_string("public/index.html").await {
        Ok(content) => Html(content).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load page").into_response(),
    }
}

pub async fn serve_page(page: &str) -> impl IntoResponse {
    let path = format!("public/{}.html", page);
    match fs::read_to_string(&path).await {
        Ok(content) => Html(content).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Page not found").into_response(),
    }
}

pub async fn dashboard(State(_state): State<AppState>) -> impl IntoResponse {
    serve_page("dashboard").await
}

pub async fn stats(State(_state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "total_cards": 0,
        "total_reviews": 0,
        "mastered_words": 0,
        "current_streak": 0,
        "due_count": 0
    }))
}

pub async fn profile() -> impl IntoResponse {
    (StatusCode::OK, "User profile")
}

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "vocai",
        "version": "0.1.0"
    }))
}

pub async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Page not found")
}

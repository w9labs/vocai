pub mod auth;
pub mod flashcards;
pub mod islands;
pub mod review;

// Re-export main handlers
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::models::AppState;

pub async fn index() -> impl IntoResponse {
    (StatusCode::OK, "Vocai: Vocab+AI - Learn vocabulary with AI and spaced repetition")
}

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "vocai",
        "version": "0.1.0"
    }))
}

pub async fn dashboard(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    (StatusCode::OK, "Dashboard - Your vocabulary learning hub")
}

pub async fn stats(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "total_cards": 0,
        "total_reviews": 0,
        "mastered_words": 0,
        "current_streak": 0
    }))
}

pub async fn profile() -> impl IntoResponse {
    (StatusCode::OK, "User profile")
}

pub async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Page not found")
}

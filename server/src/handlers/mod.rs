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
        Ok(c) => Html(c),
        Err(_) => Html("<h1>Loading...</h1>".into()),
    }
}

async fn serve_html(page: &str) -> axum::response::Response {
    let path = format!("public/{}.html", page);
    match fs::read_to_string(&path).await {
        Ok(c) => Html(c).into_response(),
        Err(e) => {
            tracing::error!("Failed to load {}: {}", page, e);
            (StatusCode::NOT_FOUND, Html("Page not found")).into_response()
        }
    }
}

pub async fn dashboard(State(_state): State<AppState>) -> axum::response::Response {
    serve_html("dashboard").await
}

pub async fn flashcards_page(State(_state): State<AppState>) -> axum::response::Response {
    serve_html("flashcards").await
}

pub async fn flashcards_new_page(State(_state): State<AppState>) -> axum::response::Response {
    serve_html("flashcards-new").await
}

pub async fn islands_page(State(_state): State<AppState>) -> axum::response::Response {
    serve_html("islands").await
}

pub async fn review_page(State(_state): State<AppState>) -> axum::response::Response {
    serve_html("review").await
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

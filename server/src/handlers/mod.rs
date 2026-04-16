pub mod auth;
pub mod flashcards;
pub mod islands;
pub mod review;

use crate::models::AppState;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use tokio::fs;

pub async fn index() -> impl IntoResponse {
    match fs::read_to_string("public/index.html").await {
        Ok(c) => Html(c),
        Err(_) => Html("<h1>Loading...</h1>".into()),
    }
}

pub async fn serve_html(page: &str) -> axum::response::Response {
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

pub async fn islands_new_page(State(_state): State<AppState>) -> axum::response::Response {
    serve_html("islands-new").await
}

pub async fn review_page(State(_state): State<AppState>) -> axum::response::Response {
    serve_html("review").await
}

pub async fn stats(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let (user_id, _token) = match crate::handlers::auth::get_session(&headers) {
        Some(s) => s,
        None => {
            return Json(serde_json::json!({
                "total_cards": 0, "total_reviews": 0, "mastered_words": 0,
                "current_streak": 0, "due_count": 0, "longest_streak": 0,
            }));
        }
    };

    match state.db.get().await {
        Ok(client) => {
            let total_cards = client
                .query_opt(
                    "SELECT count(*)::int FROM flashcards WHERE user_id = $1",
                    &[&user_id],
                )
                .await
                .ok()
                .and_then(|r| r.map(|r| r.get::<_, i32>(0)))
                .unwrap_or(0);

            let total_reviews = client
                .query_opt(
                    "SELECT count(*)::int FROM study_sessions WHERE user_id = $1",
                    &[&user_id],
                )
                .await
                .ok()
                .and_then(|r| r.map(|r| r.get::<_, i32>(0)))
                .unwrap_or(0);

            let mastered = client
                .query_opt(
                    "SELECT count(*)::int FROM srs_reviews WHERE user_id = $1 AND leitner_box >= 4",
                    &[&user_id],
                )
                .await
                .ok()
                .and_then(|r| r.map(|r| r.get::<_, i32>(0)))
                .unwrap_or(0);

            let due = client.query_opt(
                "SELECT count(*)::int FROM srs_reviews WHERE user_id = $1 AND next_review <= NOW()", &[&user_id],
            ).await.ok().and_then(|r| r.map(|r| r.get::<_, i32>(0))).unwrap_or(0);

            Json(serde_json::json!({
                "total_cards": total_cards,
                "total_reviews": total_reviews,
                "mastered_words": mastered,
                "current_streak": 0,
                "longest_streak": 0,
                "due_count": due,
            }))
        }
        Err(_) => Json(serde_json::json!({
            "total_cards": 0, "total_reviews": 0, "mastered_words": 0,
            "current_streak": 0, "due_count": 0, "longest_streak": 0,
        })),
    }
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

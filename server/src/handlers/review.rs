use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tracing;
use crate::models::AppState;

pub async fn session(State(_state): State<AppState>) -> impl IntoResponse {
    super::serve_html("review").await
}

pub async fn next_card(State(state): State<AppState>) -> impl IntoResponse {
    // Get next card for review
    (StatusCode::OK, "Next review card")
}

pub async fn answer(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Process review answer
    let flashcard_id = payload.get("flashcard_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    let quality = payload.get("quality")
        .and_then(|v| v.as_i64())
        .unwrap_or(3) as i32;

    tracing::info!("Review answer for card {} with quality {}", flashcard_id, quality);

    // Process with hybrid SRS algorithm
    let default_review = crate::models::SrsReview {
        id: uuid::Uuid::nil(),
        user_id: uuid::Uuid::nil(),
        flashcard_id: uuid::Uuid::parse_str(flashcard_id).unwrap_or(uuid::Uuid::nil()),
        easiness_factor: 2.5,
        interval_days: 0,
        repetitions: 0,
        next_review: chrono::Utc::now(),
        last_review: None,
        leitner_box: 1,
        created_at: chrono::Utc::now(),
    };

    let (ef, interval, reps, new_box, next_review) = 
        crate::srs::HybridSrs::process_review(&default_review, quality);

    Json(json!({
        "success": true,
        "easiness_factor": ef,
        "interval_days": interval,
        "repetitions": reps,
        "leitner_box": new_box,
        "next_review": next_review.to_rfc3339()
    }))
}

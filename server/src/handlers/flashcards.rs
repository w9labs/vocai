use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Form,
    Json,
};
use serde_json::json;
use tracing;
use crate::models::{AppState, GeneratedFlashcard};

pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    // List user's flashcards
    (StatusCode::OK, "Your Flashcards")
}

pub async fn new_form() -> impl IntoResponse {
    // Show form to create new flashcard or generate with AI
    (StatusCode::OK, "Create New Flashcard")
}

pub async fn generate(
    State(state): State<AppState>,
    Form(params): Form<serde_json::Value>,
) -> impl IntoResponse {
    let topic = params.get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("general")
        .to_string();
    
    let count = params.get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    
    let language = params.get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("English")
        .to_string();
    
    let difficulty = params.get("difficulty")
        .and_then(|v| v.as_str())
        .unwrap_or("intermediate")
        .to_string();

    tracing::info!("Generating {} flashcards for topic: {}", count, topic);

    match state.nvidia.generate_flashcards(&topic, count, &language, &difficulty).await {
        Ok(flashcards) => {
            tracing::info!("Successfully generated {} flashcards", flashcards.len());
            
            // In production, save to database here
            // For now, return as JSON
            (StatusCode::OK, Json(json!({
                "success": true,
                "count": flashcards.len(),
                "flashcards": flashcards
            })))
        }
        Err(e) => {
            tracing::error!("Failed to generate flashcards: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
    }
}

pub async fn view(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // View single flashcard
    (StatusCode::OK, format!("Flashcard: {}", id))
}

pub async fn study(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Study mode for a flashcard
    (StatusCode::OK, format!("Study mode for flashcard: {}", id))
}

pub async fn review(
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<crate::models::ReviewAnswer>,
) -> impl IntoResponse {
    // Process review answer and update SRS
    tracing::info!("Review answer for card {} with quality {}", payload.flashcard_id, payload.quality);

    // Get existing SRS review for this card
    // In production, fetch from DB. For now, use defaults.
    let default_review = crate::models::SrsReview {
        id: uuid::Uuid::nil(),
        user_id: uuid::Uuid::nil(),
        flashcard_id: payload.flashcard_id,
        easiness_factor: 2.5,
        interval_days: 0,
        repetitions: 0,
        next_review: chrono::Utc::now(),
        last_review: None,
        leitner_box: 1,
        created_at: chrono::Utc::now(),
    };

    let (ef, interval, reps, new_box, next_review) = 
        crate::srs::HybridSrs::process_review(&default_review, payload.quality);

    // Save study session
    // In production, insert into study_sessions table

    Json(json!({
        "success": true,
        "easiness_factor": ef,
        "interval_days": interval,
        "repetitions": reps,
        "leitner_box": new_box,
        "next_review": next_review
    }))
}

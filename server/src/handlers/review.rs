use crate::models::AppState;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use tracing;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ReviewAnswer {
    pub flashcard_id: String,
    pub quality: i32,
    #[serde(default)]
    pub response_time_ms: Option<i32>,
}

pub async fn session(State(_state): State<AppState>) -> impl IntoResponse {
    super::serve_html("review").await
}

pub async fn next_card(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id = match crate::handlers::auth::get_session(&headers).map(|(u, _)| u) {
        Some(uid) => uid,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Not authenticated"})),
            );
        }
    };

    match state.db.get().await {
        Ok(client) => {
            let row = client.query_opt(
                "SELECT f.id, f.word, f.definition, f.example_sentence, f.phonetic, f.part_of_speech, f.image_url, f.image_prompt, f.image_model,
                        s.easiness_factor, s.interval_days, s.repetitions, s.leitner_box
                 FROM flashcards f
                 JOIN srs_reviews s ON s.flashcard_id = f.id
                 WHERE f.user_id = $1 AND s.next_review <= NOW()
                 ORDER BY s.next_review ASC LIMIT 1",
                &[&user_id],
            ).await;

            match row {
                Ok(Some(row)) => (
                    StatusCode::OK,
                    Json(json!({
                        "id": row.get::<_, Uuid>("id").to_string(),
                        "word": row.get::<_, String>("word"),
                        "definition": row.get::<_, String>("definition"),
                        "example_sentence": row.get::<_, Option<String>>("example_sentence"),
                        "phonetic": row.get::<_, Option<String>>("phonetic"),
                        "part_of_speech": row.get::<_, Option<String>>("part_of_speech"),
                        "image_url": row.get::<_, Option<String>>("image_url"),
                        "image_prompt": row.get::<_, Option<String>>("image_prompt"),
                        "image_model": row.get::<_, Option<String>>("image_model"),
                    })),
                ),
                Ok(None) => (StatusCode::OK, Json(json!({"done": true}))),
                Err(e) => {
                    tracing::error!("DB error in next_card: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": "Database error"})),
                    )
                }
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

pub async fn answer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReviewAnswer>,
) -> (StatusCode, Json<serde_json::Value>) {
    let user_id = match crate::handlers::auth::get_session(&headers).map(|(u, _)| u) {
        Some(uid) => uid,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Not authenticated"})),
            );
        }
    };

    let flashcard_id = match Uuid::parse_str(&payload.flashcard_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid flashcard_id"})),
            );
        }
    };

    let quality = payload.quality.clamp(0, 5);

    match state.db.get().await {
        Ok(client) => {
            let srs_row = client.query_opt(
                "SELECT easiness_factor, interval_days, repetitions, leitner_box FROM srs_reviews WHERE user_id = $1 AND flashcard_id = $2",
                &[&user_id, &flashcard_id],
            ).await;

            let (ef, interval, reps, new_box, _next_review) = match srs_row {
                Ok(Some(row)) => {
                    let ef: f64 = row.get("easiness_factor");
                    let interval_days: i32 = row.get("interval_days");
                    let repetitions: i32 = row.get("repetitions");
                    let leitner_box: i32 = row.get("leitner_box");
                    crate::srs::HybridSrs::process_review_quality(
                        ef,
                        interval_days,
                        repetitions,
                        leitner_box,
                        quality,
                    )
                }
                _ => crate::srs::HybridSrs::process_review_quality(2.5, 0, 0, 1, quality),
            };

            let now = chrono::Utc::now();
            let now_str = now.to_rfc3339();
            let _ = client.execute(
                "UPDATE srs_reviews SET easiness_factor = $1, interval_days = $2, repetitions = $3,
                 leitner_box = $4, next_review = $5::timestamptz, last_review = $6::timestamptz
                 WHERE user_id = $7 AND flashcard_id = $8",
                &[&ef, &interval, &reps, &new_box, &now_str, &now_str, &user_id, &flashcard_id],
            ).await;

            let _ = client
                .execute(
                    "INSERT INTO study_sessions (user_id, flashcard_id, quality, response_time_ms)
                 VALUES ($1, $2, $3, $4)",
                    &[&user_id, &flashcard_id, &quality, &payload.response_time_ms],
                )
                .await;

            // Update streak
            let _ = client
                .execute(
                    "INSERT INTO user_stats (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
                    &[&user_id],
                )
                .await;

            let today_str = now.format("%Y-%m-%d").to_string();
            let _ = client.execute(
                "UPDATE user_stats SET
                    total_cards = (SELECT count(*)::int FROM flashcards WHERE user_id = $1),
                    total_reviews = total_reviews + 1,
                    mastered_words = (SELECT count(*)::int FROM srs_reviews WHERE user_id = $1 AND leitner_box >= 4),
                    current_streak = CASE
                        WHEN last_review_date IS NULL THEN 1
                        WHEN last_review_date = $2::date THEN current_streak
                        WHEN last_review_date = $2::date - INTERVAL '1 day' THEN current_streak + 1
                        ELSE 1 END,
                    longest_streak = GREATEST(longest_streak,
                        CASE
                            WHEN last_review_date IS NULL THEN 1
                            WHEN last_review_date = $2::date THEN current_streak
                            WHEN last_review_date = $2::date - INTERVAL '1 day' THEN current_streak + 1
                            ELSE 1 END),
                    last_review_date = $2::date
                 WHERE user_id = $1",
                &[&user_id, &today_str],
            ).await;

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "easiness_factor": ef,
                    "interval_days": interval,
                    "repetitions": reps,
                    "leitner_box": new_box,
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

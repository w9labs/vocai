use axum::{
    extract::{Path, State, Request},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    Json,
    Form,
};
use serde_json::json;
use tracing;
use uuid::Uuid;
use crate::models::AppState;
use crate::nvidia::Model;

pub async fn save(
    State(state): State<AppState>,
    req: Request,
) -> (StatusCode, Json<serde_json::Value>) {
    let headers = req.headers();
    let (user_id, _token) = match crate::handlers::auth::get_session(headers) {
        Some(s) => s,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Not authenticated"}))),
    };

    // Parse body from request
    let body = axum::body::to_bytes(req.into_body(), 1024 * 1024).await;
    let payload: serde_json::Value = match body {
        Ok(b) => serde_json::from_slice(&b).unwrap_or(serde_json::json!({})),
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid body"}))),
    };

    let word = payload.get("word").and_then(|v| v.as_str()).unwrap_or("");
    let definition = payload.get("definition").and_then(|v| v.as_str()).unwrap_or("");
    let example = payload.get("example_sentence").and_then(|v| v.as_str()).map(|s| s.to_string());
    let phonetic = payload.get("phonetic").and_then(|v| v.as_str()).map(|s| s.to_string());
    let pos = payload.get("part_of_speech").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Generate image URL via Pollinations (flux-schnell, free, no key needed)
    let image_prompt = payload.get("image_prompt")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("educational illustration for the word '{}', clean simple modern style", word));
    let image_url = state.pollinations.generate_image_url(&image_prompt);

    // Generate TTS audio URL via Google Translate TTS (free, no key needed)
    let lang = payload.get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("en");
    let tts_text = example.as_deref().unwrap_or_else(|| word);
    let tts_url = crate::pollinations::TTSClient::generate_audio_url(tts_text, lang);

    let card_id = Uuid::new_v4();
    let client = match state.db.get().await {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    };

    let result = client.execute(
        "INSERT INTO flashcards (id, user_id, word, definition, example_sentence, phonetic, part_of_speech, image_url) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[&card_id, &user_id, &word, &definition, &example, &phonetic, &pos, &image_url],
    ).await;

    match result {
        Ok(_) => {
            let _ = client.execute(
                "INSERT INTO srs_reviews (user_id, flashcard_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&user_id, &card_id],
            ).await;
            tracing::info!("Flashcard saved: {} img={} tts={}", word, image_url, tts_url);
            (StatusCode::OK, Json(json!({
                "success": true,
                "id": card_id.to_string(),
                "word": word,
                "image_url": image_url,
                "tts_url": tts_url,
            })))
        }
        Err(e) => {
            tracing::error!("DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let (user_id, _token) = match crate::handlers::auth::get_session(&headers) {
        Some(s) => s,
        None => return Json(json!({"cards": []})),
    };

    match state.db.get().await {
        Ok(client) => {
            let rows = client.query(
                "SELECT id, word, definition, example_sentence, phonetic, part_of_speech, image_url, tts_url FROM flashcards WHERE user_id = $1 ORDER BY created_at DESC",
                &[&user_id],
            ).await;

            match rows {
                Ok(rows) => {
                    let cards: Vec<serde_json::Value> = rows.iter().map(|r| json!({
                        "id": r.get::<_, Uuid>("id").to_string(),
                        "word": r.get::<_, String>("word"),
                        "definition": r.get::<_, String>("definition"),
                        "example_sentence": r.get::<_, Option<String>>("example_sentence"),
                        "phonetic": r.get::<_, Option<String>>("phonetic"),
                        "part_of_speech": r.get::<_, Option<String>>("part_of_speech"),
                        "image_url": r.get::<_, Option<String>>("image_url"),
                        "tts_url": r.get::<_, Option<String>>("tts_url"),
                    })).collect();
                    Json(json!({"cards": cards}))
                }
                Err(e) => {
                    tracing::error!("DB error listing cards: {}", e);
                    Json(json!({"cards": []}))
                }
            }
        }
        Err(_) => Json(json!({"cards": []})),
    }
}

pub async fn new_form(State(_state): State<AppState>) -> impl IntoResponse {
    super::serve_html("flashcards-new").await
}

pub async fn generate(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(params): Form<serde_json::Value>,
) -> impl IntoResponse {
    let (user_id, _token) = match crate::handlers::auth::get_session(&headers) {
        Some(s) => s,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"success": false, "error": "Not authenticated"}))),
    };

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

    let model_str = params.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("minimax");
    let model = Model::from_str(model_str);

    // Rate limiting: 20 AI requests per user per hour
    let user_id_str = user_id.to_string();
    if !state.check_ai_rate_limit(&user_id_str).await {
        tracing::warn!("Rate limit exceeded for user {}", user_id);
        return (StatusCode::TOO_MANY_REQUESTS, Json(json!({
            "success": false,
            "error": "Rate limit exceeded. Please wait before generating more flashcards."
        })));
    }

    tracing::info!("Generating {} flashcards for topic: {} using model: {}", count, topic, model.id());

    // Create a client with the selected model
    let api_key = std::env::var("NVIDIA_API_KEY").expect("NVIDIA_API_KEY must be set");
    let client = crate::nvidia::NvidiaClient::new(&api_key).with_model(model);

    match tokio::time::timeout(
        std::time::Duration::from_secs(180),
        client.generate_flashcards(&topic, count, &language, &difficulty),
    ).await {
        Ok(Ok(flashcards)) => {
            tracing::info!("Successfully generated {} flashcards", flashcards.len());
            (StatusCode::OK, Json(json!({
                "success": true,
                "count": flashcards.len(),
                "flashcards": flashcards
            })))
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to generate flashcards: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "success": false,
                "error": e.to_string()
            })))
        }
        Err(_) => {
            tracing::error!("Flashcard generation timed out after 180s");
            (StatusCode::GATEWAY_TIMEOUT, Json(json!({
                "success": false,
                "error": "AI generation timed out (3min). Try fewer words or a simpler topic."
            })))
        }
    }
}

pub async fn view(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let card_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid card ID"}))).into_response(),
    };

    match state.db.get().await {
        Ok(client) => {
            let row = client.query_opt(
                "SELECT id, word, definition, example_sentence, phonetic, part_of_speech FROM flashcards WHERE id = $1",
                &[&card_id],
            ).await;

            match row {
                Ok(Some(row)) => (StatusCode::OK, Json(json!({
                    "id": row.get::<_, Uuid>("id").to_string(),
                    "word": row.get::<_, String>("word"),
                    "definition": row.get::<_, String>("definition"),
                    "example_sentence": row.get::<_, Option<String>>("example_sentence"),
                    "phonetic": row.get::<_, Option<String>>("phonetic"),
                    "part_of_speech": row.get::<_, Option<String>>("part_of_speech"),
                }))).into_response(),
                Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "Flashcard not found"}))).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn migrate(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let (user_id, _token) = match crate::handlers::auth::get_session(&headers) {
        Some(s) => s,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Not authenticated"}))),
    };

    let client = match state.db.get().await {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    };

    // Only migrate cards belonging to THIS user
    let rows = match client.query(
        "SELECT id, word, definition, example_sentence, part_of_speech, image_url, tts_url FROM flashcards WHERE user_id = $1 AND (image_url IS NULL OR tts_url IS NULL)",
        &[&user_id],
    ).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    };

    let total = rows.len();
    let mut updated_images = 0;
    let mut updated_tts = 0;

    for row in rows {
        let id: Uuid = row.get("id");
        let word: String = row.get("word");
        let example: Option<String> = row.get("example_sentence");
        let pos: Option<String> = row.get("part_of_speech");
        let existing_image: Option<String> = row.get("image_url");
        let existing_tts: Option<String> = row.get("tts_url");

        if existing_image.is_none() {
            let pos_str = pos.as_deref().unwrap_or("word");
            let prompt = format!("educational illustration for the word '{}', {} context, clean simple modern style", word, pos_str);
            let image_url = state.pollinations.generate_image_url(&prompt);
            let _ = client.execute(
                "UPDATE flashcards SET image_url = $1, updated_at = NOW() WHERE id = $2 AND user_id = $3",
                &[&image_url, &id, &user_id],
            ).await;
            updated_images += 1;
        }

        if existing_tts.is_none() {
            let tts_text = example.as_deref().unwrap_or_else(|| &word);
            let tts_url = crate::pollinations::TTSClient::generate_audio_url(tts_text, "en");
            let _ = client.execute(
                "UPDATE flashcards SET tts_url = $1, updated_at = NOW() WHERE id = $2 AND user_id = $3",
                &[&tts_url, &id, &user_id],
            ).await;
            updated_tts += 1;
        }
    }

    tracing::info!("Migration for user {}: {} cards, {} images, {} TTS", user_id, total, updated_images, updated_tts);
    (StatusCode::OK, Json(json!({
        "success": true,
        "total_cards": total,
        "images_added": updated_images,
        "tts_added": updated_tts,
    })))
}

pub async fn study(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    (StatusCode::OK, format!("Study mode for flashcard: {}", id))
}

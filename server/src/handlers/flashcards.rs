use axum::{
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tracing;
use uuid::Uuid;

use crate::models::AppState;
use crate::nvidia::Model;

#[derive(Debug, Deserialize)]
pub struct SaveRequest {
    word: String,
    definition: String,
    example_sentence: Option<String>,
    phonetic: Option<String>,
    part_of_speech: Option<String>,
    image_prompt: Option<String>,
    image_model: Option<String>,
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
    topic: Option<String>,
    count: Option<usize>,
    language: Option<String>,
    difficulty: Option<String>,
    model: Option<String>,
}

struct ImageGenerationOutcome {
    url: Option<String>,
    model: String,
    error: Option<String>,
}

fn build_image_prompt(
    word: &str,
    definition: &str,
    example_sentence: Option<&str>,
    part_of_speech: Option<&str>,
    provided: Option<&str>,
) -> String {
    let trimmed = provided.map(str::trim).filter(|value| !value.is_empty());
    if let Some(prompt) = trimmed {
        return prompt.to_string();
    }

    let mut prompt = format!(
        "A clean educational illustration that helps remember the vocabulary word \"{}\"",
        word.trim()
    );

    let definition = definition.trim();
    if !definition.is_empty() {
        prompt.push_str(&format!(", meaning \"{}\"", definition));
    }

    if let Some(example) = example_sentence.map(str::trim).filter(|value| !value.is_empty()) {
        prompt.push_str(&format!(", shown in the context of \"{}\"", example));
    }

    if let Some(pos) = part_of_speech.map(str::trim).filter(|value| !value.is_empty()) {
        prompt.push_str(&format!(", part of speech {}", pos));
    }

    prompt.push_str(". Simple scene, no text, no labels, mnemonic style.");
    prompt
}

fn build_tts_url(text: &str, language: &str) -> String {
    crate::pollinations::TTSClient::generate_audio_url(text, language)
}

async fn generate_flashcard_image(
    state: &AppState,
    user_id: &Uuid,
    prompt: &str,
    requested_model: Option<&str>,
) -> ImageGenerationOutcome {
    let resolved_model = requested_model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(state.pollinations.default_model())
        .to_string();

    let user_key = user_id.to_string();
    if !state.check_image_rate_limit(&user_key).await {
        tracing::warn!(user_id = %user_id, "Image generation rate limit exceeded");
        return ImageGenerationOutcome {
            url: None,
            model: resolved_model,
            error: Some("Image generation rate limit exceeded".to_string()),
        };
    }

    match tokio::time::timeout(
        Duration::from_secs(45),
        state.pollinations.generate_image(prompt, Some(&resolved_model), Some(&user_key)),
    )
    .await
    {
        Ok(Ok(artifact)) => ImageGenerationOutcome {
            url: Some(artifact.url),
            model: artifact.model,
            error: None,
        },
        Ok(Err(e)) => {
            tracing::warn!(user_id = %user_id, error = %e, "Pollinations image generation failed");
            ImageGenerationOutcome {
                url: None,
                model: resolved_model,
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            tracing::warn!(user_id = %user_id, "Pollinations image generation timed out");
            ImageGenerationOutcome {
                url: None,
                model: resolved_model,
                error: Some("Image generation timed out after 45s".to_string()),
            }
        }
    }
}

pub async fn save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SaveRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let (user_id, _token) = match crate::handlers::auth::get_session(&headers) {
        Some(session) => session,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Not authenticated"}))),
    };

    let word = payload.word.trim();
    let definition = payload.definition.trim();
    if word.is_empty() || definition.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "word and definition are required"})),
        );
    }

    let image_prompt = build_image_prompt(
        word,
        definition,
        payload.example_sentence.as_deref(),
        payload.part_of_speech.as_deref(),
        payload.image_prompt.as_deref(),
    );
    let resolved_image_model = payload
        .image_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(state.pollinations.default_model())
        .to_string();
    let tts_lang = payload.language.as_deref().unwrap_or("en");
    let tts_text = payload.example_sentence.as_deref().unwrap_or(word);
    let tts_url = build_tts_url(tts_text, tts_lang);

    let image = generate_flashcard_image(
        &state,
        &user_id,
        &image_prompt,
        Some(&resolved_image_model),
    )
    .await;

    let card_id = Uuid::new_v4();
    let client = match state.db.get().await {
        Ok(client) => client,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    };

    let image_url = image.url.clone();
    let image_model = Some(image.model.clone());
    let result = client
        .execute(
            "INSERT INTO flashcards (id, user_id, word, definition, example_sentence, image_url, image_prompt, image_model, tts_url, phonetic, part_of_speech) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            &[
                &card_id,
                &user_id,
                &word,
                &definition,
                &payload.example_sentence,
                &image_url,
                &image_prompt,
                &image_model,
                &tts_url,
                &payload.phonetic,
                &payload.part_of_speech,
            ],
        )
        .await;

    match result {
        Ok(_) => {
            let mut srs_warning = None;
            if let Err(e) = client
                .execute(
                    "INSERT INTO srs_reviews (user_id, flashcard_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    &[&user_id, &card_id],
                )
                .await
            {
                tracing::warn!(user_id = %user_id, flashcard_id = %card_id, error = %e, "Failed to initialize SRS row");
                srs_warning = Some(e.to_string());
            }
            tracing::info!(
                "Flashcard saved: {} image={} tts={}",
                word,
                image_url.as_deref().unwrap_or("none"),
                tts_url
            );
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "id": card_id.to_string(),
                    "word": word,
                    "image_url": image_url,
                    "image_prompt": image_prompt,
                    "image_model": image_model,
                    "image_generated": image.url.is_some(),
                    "image_error": image.error,
                    "tts_url": tts_url,
                    "srs_warning": srs_warning,
                })),
            )
        }
        Err(e) => {
            tracing::error!("DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_id, _token) = match crate::handlers::auth::get_session(&headers) {
        Some(session) => session,
        None => return Json(json!({"cards": []})),
    };

    match state.db.get().await {
        Ok(client) => {
            let rows = client
                .query(
                    "SELECT id, word, definition, example_sentence, phonetic, part_of_speech, image_url, image_prompt, image_model, tts_url FROM flashcards WHERE user_id = $1 ORDER BY created_at DESC",
                    &[&user_id],
                )
                .await;

            match rows {
                Ok(rows) => {
                    let cards: Vec<serde_json::Value> = rows
                        .iter()
                        .map(|row| {
                            json!({
                                "id": row.get::<_, Uuid>("id").to_string(),
                                "word": row.get::<_, String>("word"),
                                "definition": row.get::<_, String>("definition"),
                                "example_sentence": row.get::<_, Option<String>>("example_sentence"),
                                "phonetic": row.get::<_, Option<String>>("phonetic"),
                                "part_of_speech": row.get::<_, Option<String>>("part_of_speech"),
                                "image_url": row.get::<_, Option<String>>("image_url"),
                                "image_prompt": row.get::<_, Option<String>>("image_prompt"),
                                "image_model": row.get::<_, Option<String>>("image_model"),
                                "tts_url": row.get::<_, Option<String>>("tts_url"),
                            })
                        })
                        .collect();
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
    headers: HeaderMap,
    Form(params): Form<GenerateRequest>,
) -> impl IntoResponse {
    let (user_id, _token) = match crate::handlers::auth::get_session(&headers) {
        Some(session) => session,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"success": false, "error": "Not authenticated"})),
            )
        }
    };

    let topic = params.topic.unwrap_or_else(|| "general".to_string());
    let count = params.count.unwrap_or(10).clamp(1, 20);
    let language = params.language.unwrap_or_else(|| "English".to_string());
    let difficulty = params.difficulty.unwrap_or_else(|| "intermediate".to_string());
    let model_str = params.model.unwrap_or_else(|| "minimax".to_string());
    let model = Model::from_str(&model_str);

    let user_id_str = user_id.to_string();
    if !state.check_ai_rate_limit(&user_id_str).await {
        tracing::warn!("Rate limit exceeded for user {}", user_id);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "success": false,
                "error": "Rate limit exceeded. Please wait before generating more flashcards."
            })),
        );
    }

    tracing::info!(
        "Generating {} flashcards for topic: {} using model: {}",
        count,
        topic,
        model.id()
    );

    let api_key = std::env::var("NVIDIA_API_KEY").expect("NVIDIA_API_KEY must be set");
    let client = crate::nvidia::NvidiaClient::new(&api_key).with_model(model);

    match tokio::time::timeout(
        Duration::from_secs(180),
        client.generate_flashcards(&topic, count, &language, &difficulty),
    )
    .await
    {
        Ok(Ok(flashcards)) => {
            tracing::info!("Successfully generated {} flashcards", flashcards.len());
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "count": flashcards.len(),
                    "flashcards": flashcards
                })),
            )
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to generate flashcards: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": e.to_string()
                })),
            )
        }
        Err(_) => {
            tracing::error!("Flashcard generation timed out after 180s");
            (
                StatusCode::GATEWAY_TIMEOUT,
                Json(json!({
                    "success": false,
                    "error": "AI generation timed out (3min). Try fewer words or a simpler topic."
                })),
            )
        }
    }
}

pub async fn view(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let card_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid card ID"})),
            )
                .into_response()
        }
    };

    match state.db.get().await {
        Ok(client) => {
            let row = client
                .query_opt(
                    "SELECT id, word, definition, example_sentence, phonetic, part_of_speech, image_url, image_prompt, image_model FROM flashcards WHERE id = $1",
                    &[&card_id],
                )
                .await;

            match row {
                Ok(Some(row)) => (StatusCode::OK, Json(json!({
                    "id": row.get::<_, Uuid>("id").to_string(),
                    "word": row.get::<_, String>("word"),
                    "definition": row.get::<_, String>("definition"),
                    "example_sentence": row.get::<_, Option<String>>("example_sentence"),
                    "phonetic": row.get::<_, Option<String>>("phonetic"),
                    "part_of_speech": row.get::<_, Option<String>>("part_of_speech"),
                    "image_url": row.get::<_, Option<String>>("image_url"),
                    "image_prompt": row.get::<_, Option<String>>("image_prompt"),
                    "image_model": row.get::<_, Option<String>>("image_model"),
                })))
                .into_response(),
                Ok(None) => (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "Flashcard not found"})),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn migrate(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let (user_id, _token) = match crate::handlers::auth::get_session(&headers) {
        Some(session) => session,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Not authenticated"}))),
    };

    let client = match state.db.get().await {
        Ok(client) => client,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    };

    let rows = match client
        .query(
            "SELECT id, word, definition, example_sentence, part_of_speech, image_url, image_prompt, image_model, tts_url FROM flashcards WHERE user_id = $1 AND (image_url IS NULL OR tts_url IS NULL OR image_prompt IS NULL OR image_model IS NULL)",
            &[&user_id],
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))),
    };

    let total = rows.len();
    let mut updated_images = 0;
    let mut updated_tts = 0;
    let mut updated_prompts = 0;
    let mut image_failures = 0;
    let mut db_failures = 0;

    for row in rows {
        let id: Uuid = row.get("id");
        let word: String = row.get("word");
        let definition: String = row.get("definition");
        let example: Option<String> = row.get("example_sentence");
        let pos: Option<String> = row.get("part_of_speech");
        let existing_image: Option<String> = row.get("image_url");
        let existing_prompt: Option<String> = row.get("image_prompt");
        let existing_model: Option<String> = row.get("image_model");
        let existing_tts: Option<String> = row.get("tts_url");

        let prompt = build_image_prompt(
            &word,
            &definition,
            example.as_deref(),
            pos.as_deref(),
            existing_prompt.as_deref(),
        );
        let prompt_update = if existing_prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            updated_prompts += 1;
            Some(prompt.clone())
        } else {
            None
        };

        let model_update = if existing_model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            Some(state.pollinations.default_model().to_string())
        } else {
            None
        };

        let mut image_update: Option<String> = None;
        if existing_image.is_none() {
            let image = generate_flashcard_image(
                &state,
                &user_id,
                &prompt,
                model_update
                    .as_deref()
                    .or(existing_model.as_deref())
                    .or(Some(state.pollinations.default_model())),
            )
            .await;

            if let Some(url) = image.url {
                image_update = Some(url);
                updated_images += 1;
                if existing_model.is_none() {
                    // keep the generated model metadata aligned with the actual request
                }
            } else if let Some(error) = image.error {
                image_failures += 1;
                tracing::warn!(user_id = %user_id, flashcard_id = %id, error = %error, "Skipping Pollinations image backfill");
            }

            if existing_model.is_none() && image_update.is_some() {
                // update model metadata to the actual generated model below
            }
        }

        let tts_update = if existing_tts.is_none() {
            updated_tts += 1;
            Some(build_tts_url(example.as_deref().unwrap_or(&word), "en"))
        } else {
            None
        };

        let image_model_update = if existing_image.is_none() {
            model_update
        } else {
            None
        };

        if let Err(e) = client
            .execute(
                "UPDATE flashcards SET image_url = COALESCE($1, image_url), image_prompt = COALESCE($2, image_prompt), image_model = COALESCE($3, image_model), tts_url = COALESCE($4, tts_url), updated_at = NOW() WHERE id = $5 AND user_id = $6",
                &[&image_update, &prompt_update, &image_model_update, &tts_update, &id, &user_id],
            )
            .await
        {
            db_failures += 1;
            tracing::warn!(user_id = %user_id, flashcard_id = %id, error = %e, "Failed to update migrated flashcard");
        }
    }

    tracing::info!(
        "Migration for user {}: {} cards, {} images, {} prompts, {} TTS",
        user_id,
        total,
        updated_images,
        updated_prompts,
        updated_tts
    );

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "total_cards": total,
            "images_added": updated_images,
            "prompts_added": updated_prompts,
            "tts_added": updated_tts,
            "image_failures": image_failures,
            "db_failures": db_failures,
        })),
    )
}

pub async fn study(
    Path(id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    (StatusCode::OK, format!("Study mode for flashcard: {}", id))
}

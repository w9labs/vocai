use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: crate::db::DbPool,
    pub nvidia: crate::nvidia::NvidiaClient,
    pub pollinations: crate::pollinations::PollinationsClient,
    pub http_client: reqwest::Client,
    pub rate_limiter: RateLimiter,
}

impl AppState {
    pub async fn check_ai_rate_limit(&self, user_id: &str) -> bool {
        self.rate_limiter
            .is_allowed(&format!("ai:{}", user_id))
            .await
    }

    pub async fn check_image_rate_limit(&self, user_id: &str) -> bool {
        self.rate_limiter
            .is_allowed(&format!("image:{}", user_id))
            .await
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub oauth_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Flashcard {
    pub id: Uuid,
    pub user_id: Uuid,
    pub island_id: Option<Uuid>,
    pub word: String,
    pub definition: String,
    pub example_sentence: Option<String>,
    pub image_url: Option<String>,
    pub image_prompt: Option<String>,
    pub image_model: Option<String>,
    pub phonetic: Option<String>,
    pub part_of_speech: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SrsReview {
    pub id: Uuid,
    pub user_id: Uuid,
    pub flashcard_id: Uuid,
    pub easiness_factor: f64,
    pub interval_days: i32,
    pub repetitions: i32,
    pub next_review: chrono::DateTime<chrono::Utc>,
    pub last_review: Option<chrono::DateTime<chrono::Utc>>,
    pub leitner_box: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeneratedFlashcard {
    pub word: String,
    pub definition: String,
    pub example_sentence: String,
    pub phonetic: Option<String>,
    pub part_of_speech: String,
    pub image_prompt: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserStats {
    pub user_id: Uuid,
    pub total_cards: i32,
    pub total_reviews: i32,
    pub mastered_words: i32,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub last_review_date: Option<chrono::NaiveDate>,
}

// Rate limiter
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_secs,
        }
    }

    pub async fn is_allowed(&self, key: &str) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);
        let mut state = self.state.lock().await;
        let entries = state.entry(key.to_string()).or_default();
        entries.retain(|t| now.duration_since(*t) < window);
        if entries.len() < self.max_requests {
            entries.push(now);
            true
        } else {
            false
        }
    }
}

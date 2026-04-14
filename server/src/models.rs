use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: crate::db::DbPool,
    pub nvidia: crate::nvidia::NvidiaClient,
    pub pollinations: crate::pollinations::PollinationsClient,
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
    pub phonetic: Option<String>,
    pub part_of_speech: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VocabularyIsland {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub topic: String,
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
pub struct StudySession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub flashcard_id: Uuid,
    pub quality: i32,
    pub review_timestamp: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: Option<i32>,
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

// AI Generation structures
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeneratedFlashcard {
    pub word: String,
    pub definition: String,
    pub example_sentence: String,
    pub phonetic: Option<String>,
    pub part_of_speech: String,
    pub image_prompt: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenerateRequest {
    pub topic: String,
    pub count: Option<usize>,
    pub language: Option<String>,
    pub difficulty: Option<String>,
}

// Review quality for SM-2 algorithm
#[derive(Debug, Deserialize)]
pub struct ReviewAnswer {
    pub flashcard_id: Uuid,
    pub quality: i32, // 0-5 scale
    pub response_time_ms: Option<i32>,
}

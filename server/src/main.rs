use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, HeaderValue, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod handlers;
mod models;
mod nvidia;
mod pollinations;
mod srs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vocai_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_pool = db::init_pool(&db_url).await.expect("Failed to create DB pool");
    db::run_migrations(&db_pool).await.expect("Failed to run migrations");

    let nvidia_api_key = std::env::var("NVIDIA_API_KEY").expect("NVIDIA_API_KEY must be set");
    let pollinations_api_key = std::env::var("POLLINATIONS_API_KEY")
        .unwrap_or_else(|_| String::new());

    let nvidia_client = nvidia::NvidiaClient::new(&nvidia_api_key);
    let pollinations_client = pollinations::PollinationsClient::new(&pollinations_api_key);
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    // Rate limiter: 20 AI requests per user per hour
    let rate_limiter = crate::models::RateLimiter::new(20, 3600);

    let app_state = crate::models::AppState {
        db: db_pool,
        nvidia: nvidia_client,
        pollinations: pollinations_client,
        http_client,
        rate_limiter,
    };

    let app = Router::new()
        // Health & meta
        .route("/", get(handlers::index))
        .route("/health", get(handlers::health))
        .route("/api/health", get(handlers::health))

        // Auth
        .route("/login", get(handlers::auth::login))
        .route("/oauth/callback", get(handlers::auth::callback))
        .route("/logout", get(handlers::auth::logout))

        // Pages
        .route("/dashboard", get(handlers::dashboard))
        .route("/flashcards", get(handlers::flashcards_page))
        .route("/flashcards/new", get(handlers::flashcards_new_page))
        .route("/islands", get(handlers::islands_page))
        .route("/islands/new", get(handlers::islands_new_page))
        .route("/review", get(handlers::review_page))
        .route("/stats", get(handlers::stats))
        .route("/profile", get(handlers::profile))

        // JSON APIs
        .route("/api/flashcards", get(handlers::flashcards::list))
        .route("/api/flashcards/save", post(handlers::flashcards::save))
        .route("/api/flashcards/migrate", post(handlers::flashcards::migrate))
        .route("/api/flashcards/generate", post(handlers::flashcards::generate))

        // Review session API
        .route("/api/review/next", get(handlers::review::next_card))
        .route("/api/review/answer", post(handlers::review::answer))

        // TTS proxy (Google Translate — no CORS issues)
        .route("/api/tts", get(tts_proxy))

        // Static files
        .route_service("/favicon.ico", ServeFile::new("public/w9-logo/logo.svg"))
        .route_service("/favicon.svg", ServeFile::new("public/w9-logo/logo.svg"))
        .nest_service("/assets", ServeDir::new("public/assets"))
        .nest_service("/w9-logo", ServeDir::new("public/w9-logo"))
        .fallback_service(ServeDir::new("public"))
        .with_state(app_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3010".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🚀 Vocai server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Deserialize)]
struct TTSParams {
    text: String,
    #[serde(default = "default_lang")]
    lang: String,
}

fn default_lang() -> String { "en".to_string() }

async fn tts_proxy(
    State(state): State<crate::models::AppState>,
    Query(params): Query<TTSParams>,
) -> impl IntoResponse {
    if params.text.is_empty() {
        return (StatusCode::BAD_REQUEST, "text parameter required").into_response();
    }

    let url = format!(
        "https://translate.google.com/translate_tts?ie=UTF-8&tl={}&client=tw-ob&q={}",
        params.lang,
        urlencoding::encode(&params.text)
    );

    match state.http_client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.bytes().await.unwrap_or_default();

            let mut response = Response::new(Body::from(body));
            *response.status_mut() = status;
            response.headers_mut().insert(
                header::CONTENT_TYPE, HeaderValue::from_static("audio/mpeg"),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=86400"),
            );
            response.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"),
            );
            response
        }
        Err(e) => {
            tracing::error!("TTS proxy error: {}", e);
            (StatusCode::BAD_GATEWAY, "TTS service unavailable").into_response()
        }
    }
}

use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod handlers;
mod models;
mod nvidia;
mod srs;
mod session;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vocai_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_pool = db::init_pool(&db_url).await.expect("Failed to create DB pool");

    db::run_migrations(&db_pool).await.expect("Failed to run migrations");

    let nvidia_api_key = std::env::var("NVIDIA_API_KEY").expect("NVIDIA_API_KEY must be set");
    let nvidia_client = nvidia::NvidiaClient::new(&nvidia_api_key);

    let app_state = models::AppState {
        db: db_pool,
        nvidia: nvidia_client,
    };

    let app = Router::new()
        // API & dynamic routes (first = higher priority)
        .route("/", get(handlers::index))
        .route("/health", get(handlers::health))
        .route("/api/health", get(handlers::health))
        .route("/login", get(handlers::auth::login))
        .route("/auth/callback", get(handlers::auth::callback))
        .route("/logout", get(handlers::auth::logout))
        .route("/dashboard", get(handlers::dashboard))
        .route("/flashcards", get(handlers::flashcards::list))
        .route("/flashcards/new", get(handlers::flashcards::new_form))
        .route("/flashcards/generate", post(handlers::flashcards::generate))
        .route("/flashcards/:id", get(handlers::flashcards::view))
        .route("/flashcards/:id/study", get(handlers::flashcards::study))
        .route("/flashcards/:id/review", post(handlers::flashcards::review))
        .route("/islands", get(handlers::islands::list))
        .route("/islands/new", get(handlers::islands::new_form))
        .route("/islands", post(handlers::islands::create))
        .route("/islands/:id", get(handlers::islands::view))
        .route("/review", get(handlers::review::session))
        .route("/review/next", get(handlers::review::next_card))
        .route("/review/answer", post(handlers::review::answer))
        .route("/stats", get(handlers::stats))
        .route("/profile", get(handlers::profile))

        // Static files (fallback)
        .route_service("/favicon.ico", ServeFile::new("public/w9-logo/logo.svg"))
        .route_service("/favicon.svg", ServeFile::new("public/w9-logo/logo.svg"))
        .nest_service("/assets", ServeDir::new("public/assets"))
        .nest_service("/w9-logo", ServeDir::new("public/w9-logo"))
        .fallback_service(ServeDir::new("public"))
        .with_state(app_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🚀 Vocai server starting on {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

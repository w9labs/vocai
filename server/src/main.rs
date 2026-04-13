use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
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
    
    // Run migrations
    db::run_migrations(&db_pool).await.expect("Failed to run migrations");

    let nvidia_api_key = std::env::var("NVIDIA_API_KEY").expect("NVIDIA_API_KEY must be set");
    let nvidia_client = nvidia::NvidiaClient::new(&nvidia_api_key);

    let app_state = models::AppState {
        db: db_pool,
        nvidia: nvidia_client,
    };

    let app = Router::new()
        // Public routes
        .route("/", get(handlers::index))
        .route("/health", get(handlers::health))
        .route("/api/health", get(handlers::health))
        
        // Auth routes (OAuth via w9-db)
        .route("/login", get(handlers::auth::login))
        .route("/auth/callback", get(handlers::auth::callback))
        .route("/logout", get(handlers::auth::logout))
        
        // Dashboard (requires auth)
        .route("/dashboard", get(handlers::dashboard))
        
        // Flashcard routes
        .route("/flashcards", get(handlers::flashcards::list))
        .route("/flashcards/new", get(handlers::flashcards::new_form))
        .route("/flashcards/generate", post(handlers::flashcards::generate))
        .route("/flashcards/:id", get(handlers::flashcards::view))
        .route("/flashcards/:id/study", get(handlers::flashcards::study))
        .route("/flashcards/:id/review", post(handlers::flashcards::review))
        
        // Vocabulary Island routes
        .route("/islands", get(handlers::islands::list))
        .route("/islands/new", get(handlers::islands::new_form))
        .route("/islands", post(handlers::islands::create))
        .route("/islands/:id", get(handlers::islands::view))
        
        // SRS review session
        .route("/review", get(handlers::review::session))
        .route("/review/next", get(handlers::review::next_card))
        .route("/review/answer", post(handlers::review::answer))
        
        // Stats & profile
        .route("/stats", get(handlers::stats))
        .route("/profile", get(handlers::profile))
        
        // Static assets
        .nest_service("/assets", tower_http::services::ServeDir::new("public"))
        .fallback(get(handlers::not_found))
        .with_state(app_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🚀 Vocai server starting on {}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

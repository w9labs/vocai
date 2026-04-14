use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use tracing;

pub type DbPool = Pool<PostgresConnectionManager<NoTls>>;

pub async fn init_pool(database_url: &str) -> Result<DbPool, Box<dyn std::error::Error + Send + Sync>> {
    // Use the database URL from environment
    let config: tokio_postgres::Config = database_url.parse()?;
    let manager = PostgresConnectionManager::new(config, NoTls);
    
    let pool = Pool::builder()
        .max_size(15)
        .build(manager)
        .await?;

    tracing::info!("✅ Database pool initialized");
    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = pool.get().await?;
    
    client.batch_execute("
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) UNIQUE NOT NULL,
            oauth_id VARCHAR(255) UNIQUE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS vocabulary_islands (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            name VARCHAR(255) NOT NULL,
            description TEXT,
            topic VARCHAR(100) NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS flashcards (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            island_id UUID REFERENCES vocabulary_islands(id) ON DELETE SET NULL,
            word VARCHAR(255) NOT NULL,
            definition TEXT NOT NULL,
            example_sentence TEXT,
            image_url TEXT,
            image_prompt TEXT,
            image_model VARCHAR(100),
            tts_url TEXT,
            phonetic VARCHAR(100),
            part_of_speech VARCHAR(50),
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS srs_reviews (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            flashcard_id UUID REFERENCES flashcards(id) ON DELETE CASCADE,
            easiness_factor DOUBLE PRECISION DEFAULT 2.5,
            interval_days INTEGER DEFAULT 0,
            repetitions INTEGER DEFAULT 0,
            next_review TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            last_review TIMESTAMP WITH TIME ZONE,
            leitner_box INTEGER DEFAULT 1,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            UNIQUE(user_id, flashcard_id)
        );

        CREATE TABLE IF NOT EXISTS study_sessions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            flashcard_id UUID REFERENCES flashcards(id) ON DELETE CASCADE,
            quality INTEGER NOT NULL CHECK (quality >= 0 AND quality <= 5),
            review_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            response_time_ms INTEGER,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS user_stats (
            user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            total_cards INTEGER DEFAULT 0,
            total_reviews INTEGER DEFAULT 0,
            mastered_words INTEGER DEFAULT 0,
            current_streak INTEGER DEFAULT 0,
            longest_streak INTEGER DEFAULT 0,
            last_review_date DATE,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_flashcards_user_id ON flashcards(user_id);
        CREATE INDEX IF NOT EXISTS idx_flashcards_island_id ON flashcards(island_id);
        CREATE INDEX IF NOT EXISTS idx_srs_reviews_user_id ON srs_reviews(user_id);
        CREATE INDEX IF NOT EXISTS idx_srs_reviews_next_review ON srs_reviews(next_review);
        CREATE INDEX IF NOT EXISTS idx_srs_reviews_leitner_box ON srs_reviews(leitner_box);
        CREATE INDEX IF NOT EXISTS idx_study_sessions_user_id ON study_sessions(user_id);
        CREATE INDEX IF NOT EXISTS idx_vocabulary_islands_user_id ON vocabulary_islands(user_id);
        -- Add tts_url column if not exists (for existing deployments)
        ALTER TABLE flashcards ADD COLUMN IF NOT EXISTS tts_url TEXT;
        ALTER TABLE flashcards ADD COLUMN IF NOT EXISTS image_prompt TEXT;
        ALTER TABLE flashcards ADD COLUMN IF NOT EXISTS image_model VARCHAR(100);
    ").await?;

    tracing::info!("✅ Database migrations completed");
    Ok(())
}

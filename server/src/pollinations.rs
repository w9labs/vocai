use reqwest::Client;
use tracing;

/// Pollinations image generator — uses API key for authenticated access
#[derive(Clone)]
pub struct PollinationsClient {
    client: Client,
    api_key: String,
}

impl PollinationsClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: api_key.to_string(),
        }
    }

    /// Generate image URL for a flashcard word.
    /// Uses authenticated endpoint with API key for higher rate limits.
    pub fn generate_image_url(&self, prompt: &str) -> String {
        format!(
            "https://image.pollinations.ai/prompt/{}?model=flux-schnell&width=512&height=512&nologo=true&token={}",
            urlencoding::encode(prompt),
            self.api_key
        )
    }

    /// Generate and download image bytes
    pub async fn generate_image(&self, prompt: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let url = self.generate_image_url(prompt);
        tracing::info!("Generating image from Pollinations: {}", prompt);

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(format!("Pollinations API error: {}", response.status()).into());
        }

        let bytes = response.bytes().await?;
        tracing::info!("Image generated: {} bytes", bytes.len());
        Ok(bytes.to_vec())
    }
}

/// Google Translate TTS — free, no API key needed
/// Uses the unofficial translate.google.com endpoint
pub struct TTSClient;

impl TTSClient {
    /// Generate TTS audio URL for text.
    /// Returns MP3 audio when accessed.
    /// lang: language code (en, es, fr, de, zh, ja, ko, vi, etc.)
    pub fn generate_audio_url(text: &str, lang: &str) -> String {
        format!(
            "https://translate.google.com/translate_tts?ie=UTF-8&tl={}&client=tw-ob&q={}",
            lang,
            urlencoding::encode(text)
        )
    }
}

use reqwest::Client;
use tracing;

/// Pollinations image generator — free, no API key needed
#[derive(Clone)]
pub struct PollinationsClient {
    client: Client,
}

impl PollinationsClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Generate image URL for a flashcard word.
    /// Returns a direct URL to the generated image (Pollinations serves it directly).
    pub fn generate_image_url(&self, prompt: &str) -> String {
        // Use flux-schnell for fast generation (~2-3s)
        format!(
            "https://image.pollinations.ai/prompt/{}?model=flux-schnell&width=512&height=512&nologo=true",
            urlencoding::encode(prompt)
        )
    }

    /// Generate and download image bytes (for caching/serving locally)
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

/// Pollinations text-to-speech — free, no API key needed
pub struct TTSClient;

impl TTSClient {
    /// Generate TTS audio URL for a word/phrase.
    /// Returns a direct URL to the generated audio (Pollinations serves it directly).
    /// The URL returns audio/mp3 when accessed.
    pub fn generate_audio_url(text: &str) -> String {
        format!(
            "https://text.pollinations.ai/{}?model=openai-audio&voice=alloy",
            urlencoding::encode(text)
        )
    }
}

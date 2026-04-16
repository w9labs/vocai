use reqwest::header::{AUTHORIZATION, HeaderValue, REFERER, USER_AGENT};
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PollinationsError {
    #[error("image prompt missing")]
    MissingPrompt,
    #[error("api key missing")]
    MissingKey,
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("api error: {0}")]
    Api(String),
    #[error("invalid response: {0}")]
    Response(String),
}

#[derive(Clone)]
pub struct PollinationsClient {
    http: reqwest::Client,
    api_key: Option<String>,
    api_base: String,
    default_model: String,
    default_size: String,
    default_quality: String,
    referrer: String,
}

#[derive(Debug, Deserialize)]
struct CreateImageResponse {
    data: Vec<CreateImageData>,
}

#[derive(Debug, Deserialize)]
struct CreateImageData {
    url: Option<String>,
    b64_json: Option<String>,
    mime_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImageArtifact {
    pub url: String,
    pub model: String,
}

impl PollinationsClient {
    pub fn new() -> Result<Self, PollinationsError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        let api_key = std::env::var("POLLINATIONS_API_KEY")
            .ok()
            .and_then(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            });

        let api_base = std::env::var("POLLINATIONS_API_BASE")
            .unwrap_or_else(|_| "https://gen.pollinations.ai".to_string());
        let default_model =
            std::env::var("POLLINATIONS_IMAGE_MODEL").unwrap_or_else(|_| "flux".to_string());
        let default_size =
            std::env::var("POLLINATIONS_IMAGE_SIZE").unwrap_or_else(|_| "512x512".to_string());
        let default_quality =
            std::env::var("POLLINATIONS_IMAGE_QUALITY").unwrap_or_else(|_| "medium".to_string());
        let referrer = std::env::var("POLLINATIONS_REFERRER")
            .or_else(|_| std::env::var("VOCAI_BASE_URL"))
            .unwrap_or_else(|_| "https://vocai.top".to_string());

        Ok(Self {
            http,
            api_key,
            api_base,
            default_model,
            default_size,
            default_quality,
            referrer,
        })
    }

    pub fn has_key(&self) -> bool {
        self.api_key.is_some()
    }

    pub fn default_model(&self) -> &str {
        &self.default_model
    }

    pub async fn generate_image(
        &self,
        prompt: &str,
        model: Option<&str>,
        user: Option<&str>,
    ) -> Result<ImageArtifact, PollinationsError> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Err(PollinationsError::MissingPrompt);
        }

        let api_key = self
            .api_key
            .as_deref()
            .ok_or(PollinationsError::MissingKey)?;
        let resolved_model = model
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&self.default_model);

        let url = format!(
            "{}/v1/images/generations",
            self.api_base.trim_end_matches('/')
        );
        let request = self
            .http
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {}", api_key))
            .header(
                REFERER,
                HeaderValue::from_str(&self.referrer)
                    .unwrap_or_else(|_| HeaderValue::from_static("https://vocai.top")),
            )
            .header(USER_AGENT, HeaderValue::from_static("vocai/1.0"))
            .json(&serde_json::json!({
                "prompt": trimmed,
                "model": resolved_model,
                "n": 1,
                "size": self.default_size,
                "quality": self.default_quality,
                "response_format": "url",
                "safe": true,
                "user": user,
            }));

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(PollinationsError::Api(format!("HTTP {}: {}", status, body)));
        }

        let CreateImageResponse { data } = response.json().await?;
        let data = data
            .into_iter()
            .next()
            .ok_or_else(|| PollinationsError::Response("missing image data".into()))?;

        let CreateImageData {
            url,
            b64_json,
            mime_type,
        } = data;

        if let Some(url) = url {
            return Ok(ImageArtifact {
                url,
                model: resolved_model.to_string(),
            });
        }

        if let Some(b64_json) = b64_json {
            let mime = mime_type.unwrap_or_else(|| "image/jpeg".to_string());
            return Ok(ImageArtifact {
                url: format!("data:{};base64,{}", mime, b64_json),
                model: resolved_model.to_string(),
            });
        }

        Err(PollinationsError::Response(
            "missing url in response".into(),
        ))
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

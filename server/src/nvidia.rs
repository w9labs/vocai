use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing;

/// Available NVIDIA NIM models
#[derive(Debug, Clone, Copy, Default)]
pub enum Model {
    #[default]
    MiniMaxM27,
    GLM4_7,
}

impl Model {
    pub fn id(&self) -> &'static str {
        match self {
            Model::MiniMaxM27 => "minimaxai/minimax-m2.7",
            Model::GLM4_7 => "z-ai/glm4.7",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "glm4.7" | "z-ai/glm4.7" => Model::GLM4_7,
            _ => Model::MiniMaxM27,
        }
    }
}

#[derive(Clone)]
pub struct NvidiaClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: Model,
}

#[derive(Debug, Deserialize)]
struct NvidiaResponse {
    choices: Vec<NvidiaChoice>,
}

#[derive(Debug, Deserialize)]
struct NvidiaChoice {
    message: NvidiaMessage,
}

#[derive(Debug, Deserialize, Serialize)]
struct NvidiaMessage {
    content: String,
}

impl NvidiaClient {
    pub fn new(api_key: &str) -> Self {
        // No client-level timeout — the handler-level timeout controls this
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");
        Self {
            client,
            api_key: api_key.to_string(),
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            model: Model::MiniMaxM27,
        }
    }

    pub fn with_model(mut self, model: Model) -> Self {
        self.model = model;
        self
    }

    async fn chat(
        &self,
        prompt: &str,
        temperature: f32,
        max_tokens: usize,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/chat/completions", self.base_url);
        tracing::info!("NVIDIA API: {} model={}", url, self.model.id());

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model.id(),
                "messages": [{"role": "user", "content": prompt}],
                "temperature": temperature,
                "top_p": 0.95,
                "max_tokens": max_tokens,
                "stream": false,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("NVIDIA API error: {} - {}", status, body);
            return Err(format!("NVIDIA API error: {} - {}", status, body).into());
        }

        let data: NvidiaResponse = response.json().await?;
        Ok(data.choices[0].message.content.clone())
    }

    pub async fn generate_flashcards(
        &self,
        topic: &str,
        count: usize,
        language: &str,
        difficulty: &str,
    ) -> Result<Vec<crate::models::GeneratedFlashcard>, Box<dyn std::error::Error>> {
        let prompt = format!(
            r#"Generate {} vocabulary words for {} language learning at {} difficulty level.
Topic: {}

For each word, provide:
1. The word
2. Clear definition
3. Example sentence showing usage in context
4. Phonetic pronunciation (if applicable)
5. Part of speech (noun, verb, adjective, etc.)

Return ONLY valid JSON array in this exact format:
[
  {{
    "word": "example",
    "definition": "a representative form or pattern",
    "example_sentence": "This sentence is an example of proper usage.",
    "phonetic": "/ɪɡˈzɑːmpəl/",
    "part_of_speech": "noun"
  }}
]

Ensure words are practical and related to the topic."#,
            count, language, difficulty, topic
        );

        let content = self.chat(&prompt, 0.7, 4000).await?;

        // Extract JSON array from response
        let json_start = content.find('[').ok_or("No JSON array found in response")?;
        let json_end = content.rfind(']').ok_or("No JSON array end found in response")?;
        let json_str = &content[json_start..=json_end];

        let flashcards: Vec<crate::models::GeneratedFlashcard> = serde_json::from_str(json_str)?;
        tracing::info!("Generated {} flashcards via {}", flashcards.len(), self.model.id());
        Ok(flashcards)
    }

    pub async fn explain_word(
        &self,
        word: &str,
        language: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            r#"Explain the word "{}" in {} for a language learner. Include etymology, common collocations, memory tricks, and cultural context."#,
            word, language
        );
        self.chat(&prompt, 0.7, 500).await
    }

    pub async fn generate_image_prompt(
        &self,
        word: &str,
        context: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            r#"Create a vivid visual description to help remember the word "{}" in this context: {}"#,
            word, context
        );
        self.chat(&prompt, 0.8, 200).await
    }
}

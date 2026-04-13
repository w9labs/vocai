use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing;

#[derive(Clone)]
pub struct NvidiaClient {
    client: Client,
    api_key: String,
    base_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct NvidiaRequest {
    messages: Vec<NvidiaMessage>,
    temperature: f32,
    top_p: f32,
    max_tokens: usize,
    stream: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct NvidiaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct NvidiaResponse {
    choices: Vec<NvidiaChoice>,
}

#[derive(Debug, Deserialize)]
struct NvidiaChoice {
    message: NvidiaMessage,
}

impl NvidiaClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
        }
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
6. A short image prompt describing a visual representation

Return ONLY valid JSON array in this exact format:
[
  {{
    "word": "example",
    "definition": "a representative form or pattern",
    "example_sentence": "This sentence is an example of proper usage.",
    "phonetic": "/ɪɡˈzɑːmpəl/",
    "part_of_speech": "noun",
    "image_prompt": "A textbook showing sample problems"
  }}
]

Ensure words are practical and related to the topic. Focus on contextual learning - words that appear together in real scenarios."#,
            count, language, difficulty, topic
        );

        let request_body = NvidiaRequest {
            messages: vec![NvidiaMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 4000,
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            tracing::error!("NVIDIA API error: {} - {}", status, error_text);
            return Err(format!("NVIDIA API error: {} - {}", status, error_text).into());
        }

        let nvidia_response: NvidiaResponse = response.json().await?;
        let content = &nvidia_response.choices[0].message.content;

        // Parse the JSON from the response
        let json_start = content.find('[').ok_or("No JSON array found in response")?;
        let json_end = content.rfind(']').ok_or("No JSON array end found in response")?;
        let json_str = &content[json_start..=json_end];

        let flashcards: Vec<crate::models::GeneratedFlashcard> = serde_json::from_str(json_str)?;
        
        tracing::info!("Generated {} flashcards from NVIDIA AI", flashcards.len());
        Ok(flashcards)
    }

    pub async fn generate_image_prompt(
        &self,
        word: &str,
        context: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            r#"Create a vivid, memorable visual description that would help someone remember the word "{}" in this context: {}

Provide a concise image prompt (max 50 words) that captures the essence of the word and makes it memorable through visual association.
Focus on concrete, visual elements that can be easily illustrated."#,
            word, context
        );

        let request_body = NvidiaRequest {
            messages: vec![NvidiaMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.8,
            top_p: 0.9,
            max_tokens: 200,
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("NVIDIA API error: {}", response.status()).into());
        }

        let nvidia_response: NvidiaResponse = response.json().await?;
        Ok(nvidia_response.choices[0].message.content.clone())
    }

    pub async fn explain_word(
        &self,
        word: &str,
        language: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            r#"Explain the word "{}" in {} for a language learner.
Include:
- Etymology/origin if helpful
- Common collocations and phrases
- Memory tricks or mnemonics
- Cultural context if relevant
- Similar words or common confusions

Keep it concise but comprehensive."#,
            word, language
        );

        let request_body = NvidiaRequest {
            messages: vec![NvidiaMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 500,
            stream: false,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("NVIDIA API error: {}", response.status()).into());
        }

        let nvidia_response: NvidiaResponse = response.json().await?;
        Ok(nvidia_response.choices[0].message.content.clone())
    }
}

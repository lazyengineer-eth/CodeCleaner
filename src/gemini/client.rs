use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::transport::rate_limiter::RateLimiter;
use crate::transport::retry::with_retry;
use reqwest::Client;

use super::types::*;

/// Google Gemini API client.
pub struct GeminiClient {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_output_tokens: u32,
    rate_limiter: RateLimiter,
}

impl GeminiClient {
    pub fn new(config: &AppConfig) -> AppResult<Self> {
        let api_key = config.gemini_api_key()?;
        let client = Client::builder().build().map_err(AppError::Http)?;

        Ok(Self {
            client,
            api_key,
            model: config.gemini.model.clone(),
            temperature: config.gemini.temperature,
            max_output_tokens: config.gemini.max_output_tokens,
            rate_limiter: RateLimiter::new(config.performance.gemini_rate_limit_rpm / 60 + 1),
        })
    }

    /// Send a prompt to Gemini and get the raw text response.
    pub async fn generate(&self, prompt: &str) -> AppResult<String> {
        self.rate_limiter.wait().await;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
                role: "user".into(),
            }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json".into(),
                temperature: self.temperature,
                max_output_tokens: self.max_output_tokens,
            },
        };

        let response = with_retry(|| {
            let req = serde_json::to_value(&request).unwrap();
            async {
                self.client
                    .post(&url)
                    .json(&req)
                    .send()
                    .await
            }
        })
        .await?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::RateLimited {
                service: "Gemini".into(),
                retry_after_secs: 60,
            });
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::GeminiApi {
                status: status.as_u16(),
                message: body,
            });
        }

        let gemini_response: GeminiResponse = response.json().await?;

        if let Some(err) = gemini_response.error {
            return Err(AppError::GeminiApi {
                status: err.code.unwrap_or(500),
                message: err.message,
            });
        }

        let text = gemini_response
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
            .ok_or_else(|| AppError::GeminiParse("Empty response from Gemini".into()))?;

        Ok(text)
    }

    /// Send a review prompt and parse the response as review comments.
    pub async fn review_code(&self, prompt: &str) -> AppResult<Vec<AiReviewComment>> {
        let text = self.generate(prompt).await?;
        Self::parse_review_response(&text)
    }

    /// Send a fix prompt and parse the response as a fix analysis.
    pub async fn analyze_fix(&self, prompt: &str) -> AppResult<FixAnalysis> {
        let text = self.generate(prompt).await?;
        Self::parse_fix_response(&text)
    }

    fn parse_review_response(text: &str) -> AppResult<Vec<AiReviewComment>> {
        // Try to extract JSON array from the response
        let trimmed = text.trim();
        let json_str = if trimmed.starts_with('[') {
            trimmed
        } else if let Some(start) = trimmed.find('[') {
            if let Some(end) = trimmed.rfind(']') {
                &trimmed[start..=end]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        serde_json::from_str(json_str).map_err(|e| {
            AppError::GeminiParse(format!("Failed to parse review response: {e}\nRaw: {text}"))
        })
    }

    fn parse_fix_response(text: &str) -> AppResult<FixAnalysis> {
        let trimmed = text.trim();
        let json_str = if trimmed.starts_with('{') {
            trimmed
        } else if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                &trimmed[start..=end]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        serde_json::from_str(json_str).map_err(|e| {
            AppError::GeminiParse(format!("Failed to parse fix response: {e}\nRaw: {text}"))
        })
    }
}

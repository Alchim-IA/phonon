use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;

const GROQ_API_ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";
const GROQ_MODEL: &str = "llama-3.3-70b-versatile";
const TIMEOUT_SECONDS: u64 = 30;
const TIMEOUT_FAST_SECONDS: u64 = 8;

/// Informations de quota Groq (mises à jour après chaque requête)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroqQuota {
    pub limit_requests: Option<u32>,
    pub remaining_requests: Option<u32>,
    pub limit_tokens: Option<u32>,
    pub remaining_tokens: Option<u32>,
    pub reset_requests: Option<String>,
    pub reset_tokens: Option<String>,
}

/// Stockage global du dernier quota connu
static LAST_QUOTA: Mutex<Option<GroqQuota>> = Mutex::new(None);

/// Récupère le dernier quota connu
pub fn get_last_quota() -> Option<GroqQuota> {
    LAST_QUOTA.lock().ok().and_then(|guard| guard.clone())
}

/// Met à jour le quota depuis les headers de réponse
fn update_quota_from_headers(headers: &reqwest::header::HeaderMap) {
    let quota = GroqQuota {
        limit_requests: headers
            .get("x-ratelimit-limit-requests")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok()),
        remaining_requests: headers
            .get("x-ratelimit-remaining-requests")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok()),
        limit_tokens: headers
            .get("x-ratelimit-limit-tokens")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok()),
        remaining_tokens: headers
            .get("x-ratelimit-remaining-tokens")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok()),
        reset_requests: headers
            .get("x-ratelimit-reset-requests")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        reset_tokens: headers
            .get("x-ratelimit-reset-tokens")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    };

    if let Ok(mut guard) = LAST_QUOTA.lock() {
        *guard = Some(quota);
    }
}

#[derive(Debug)]
pub enum GroqError {
    InvalidApiKey,
    RateLimit,
    Timeout,
    NetworkError(String),
    ParseError(String),
}

impl std::fmt::Display for GroqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroqError::InvalidApiKey => write!(f, "Invalid API key"),
            GroqError::RateLimit => write!(f, "Rate limit exceeded"),
            GroqError::Timeout => write!(f, "Request timeout"),
            GroqError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            GroqError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for GroqError {}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

pub async fn send_completion(
    api_key: &str,
    system_prompt: &str,
    text: &str,
) -> Result<String, GroqError> {
    send_completion_with_timeout(api_key, system_prompt, text, TIMEOUT_SECONDS).await
}

pub async fn send_completion_fast(
    api_key: &str,
    system_prompt: &str,
    text: &str,
) -> Result<String, GroqError> {
    send_completion_with_timeout(api_key, system_prompt, text, TIMEOUT_FAST_SECONDS).await
}

async fn send_completion_with_timeout(
    api_key: &str,
    system_prompt: &str,
    text: &str,
    timeout_secs: u64,
) -> Result<String, GroqError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| GroqError::NetworkError(e.to_string()))?;

    let request_body = ChatCompletionRequest {
        model: GROQ_MODEL.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: text.to_string(),
            },
        ],
        temperature: 0.3,
        max_tokens: 2048,
    };

    let response = client
        .post(GROQ_API_ENDPOINT)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                GroqError::Timeout
            } else {
                GroqError::NetworkError(e.to_string())
            }
        })?;

    let status = response.status();

    // Capturer les informations de quota depuis les headers
    update_quota_from_headers(response.headers());

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(GroqError::InvalidApiKey);
    }

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(GroqError::RateLimit);
    }

    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(GroqError::NetworkError(format!(
            "HTTP {}: {}",
            status, error_text
        )));
    }

    let response_body: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| GroqError::ParseError(e.to_string()))?;

    response_body
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| GroqError::ParseError("No choices in response".to_string()))
}

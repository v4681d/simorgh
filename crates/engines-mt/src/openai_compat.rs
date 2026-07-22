//! OpenAI-compatible chat-completions translator. Configurable base URL,
//! model, and key, so ANY provider (or a local llama.cpp/Ollama/LM Studio
//! server) works. Also implements `safe_prefix` — the optional LLM-assisted
//! semantic boundary used by the clause-commit policy.

use async_trait::async_trait;
use serde_json::json;
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::Translator;
use simorgh_core::types::LangPair;
use simorgh_core::{Error, Result};

pub struct OpenAiCompatTranslator {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
    is_local: bool,
}

impl OpenAiCompatTranslator {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        let base_url = choice
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".into());
        let is_local = base_url.contains("localhost") || base_url.contains("127.0.0.1");
        // Local servers usually need no key; cloud providers MUST have one —
        // surface a clear error instead of a confusing 401 later.
        let api_key = if is_local {
            crate::api_key(choice).ok()
        } else {
            Some(crate::api_key(choice).map_err(|_| Error::Engine {
                engine: "openai-compat".into(),
                message: "API key not found for this cloud provider".into(),
                fix: Some(
                    "Enter the API key in THIS card's API key field, then Save or Test".into(),
                ),
            })?)
        };
        Ok(Self {
            client: crate::http_client(choice)?,
            model: choice.model.clone().unwrap_or_else(|| "gpt-4o-mini".into()),
            api_key,
            base_url,
            is_local,
        })
    }

    async fn chat(&self, system: &str, user: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);
        tracing::info!(
            engine = "openai-compat",
            url = %url,
            model = %self.model,
            has_key = self.api_key.is_some(),
            "chat request"
        );
        let mut req = self.client.post(&url).json(&json!({
            "model": self.model,
            "temperature": 0.2,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ]
        }));
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await.map_err(|e| {
            tracing::warn!(engine = "openai-compat", error = %e, "request failed");
            err(format!("request failed: {e}"))
        })?;
        let status = resp.status();
        if !status.is_success() {
            // Surface the provider's error body — it holds the real reason
            // (bad key, unknown model, quota, etc.).
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(engine = "openai-compat", %status, body = %body, "error response");
            return Err(err(format!("HTTP {status}: {body}")));
        }
        tracing::info!(engine = "openai-compat", %status, "chat ok");
        let body: serde_json::Value = resp.json().await.map_err(|e| err(e.to_string()))?;
        Ok(body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string())
    }
}

fn err(message: String) -> Error {
    Error::Engine {
        engine: "openai-compat".into(),
        message,
        fix: Some("Check base URL, model, and API key in Settings → Engines".into()),
    }
}

#[async_trait]
impl Translator for OpenAiCompatTranslator {
    fn id(&self) -> &'static str {
        "openai-compat"
    }
    fn display_name(&self) -> String {
        if self.is_local {
            format!("{} (local server)", self.model)
        } else {
            format!("{} (API)", self.model)
        }
    }
    fn is_local(&self) -> bool {
        self.is_local
    }

    async fn translate(&self, text: &str, pair: &LangPair) -> Result<String> {
        let system = format!(
            "You are a professional simultaneous interpreter. Translate the user's text from {} to {}. \
             Output ONLY the translation — no quotes, no explanations. Preserve tone and register. \
             The input may be a partial sentence; translate what is there naturally.",
            pair.source.0, pair.target.0
        );
        self.chat(&system, text).await
    }

    async fn safe_prefix(&self, text: &str, pair: &LangPair) -> Result<Option<String>> {
        let system = format!(
            "The user is speaking {} and being interpreted into {}. Given a partial transcript, \
             return the longest prefix that can be safely translated now without waiting for more \
             words (a complete clause). Return ONLY that prefix verbatim, or an empty string if \
             nothing is safe yet.",
            pair.source.0, pair.target.0
        );
        let prefix = self.chat(&system, text).await?;
        let prefix = prefix.trim();
        if prefix.is_empty() || !text.starts_with(prefix) {
            return Ok(None);
        }
        Ok(Some(prefix.to_string()))
    }
}

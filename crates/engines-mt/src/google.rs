//! Google Cloud Translation v2 REST adapter (API-key auth).

use async_trait::async_trait;
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::Translator;
use simorgh_core::types::LangPair;
use simorgh_core::{Error, Result};

pub struct GoogleTranslator {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl GoogleTranslator {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        Ok(Self {
            client: crate::http_client(choice)?,
            base_url: choice.base_url.clone().unwrap_or_else(|| {
                "https://translation.googleapis.com/language/translate/v2".into()
            }),
            api_key: crate::api_key(choice)?,
        })
    }
}

#[async_trait]
impl Translator for GoogleTranslator {
    fn id(&self) -> &'static str {
        "google"
    }
    fn display_name(&self) -> String {
        "Google Translate (API)".into()
    }
    fn is_local(&self) -> bool {
        false
    }

    async fn translate(&self, text: &str, pair: &LangPair) -> Result<String> {
        let resp = self
            .client
            .post(&self.base_url)
            .query(&[("key", self.api_key.as_str())])
            .json(&serde_json::json!({
                "q": text,
                "source": pair.source.0,
                "target": pair.target.0,
                "format": "text",
            }))
            .send()
            .await
            .map_err(|e| err(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(err(format!("HTTP {}", resp.status())));
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| err(e.to_string()))?;
        Ok(body["data"]["translations"][0]["translatedText"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }
}

fn err(message: String) -> Error {
    Error::Engine {
        engine: "google".into(),
        message,
        fix: Some("Check your Google API key in Settings → Engines".into()),
    }
}

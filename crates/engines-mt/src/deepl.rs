//! DeepL REST v2 adapter. Base URL configurable (api-free.deepl.com vs
//! api.deepl.com, or a proxy).

use async_trait::async_trait;
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::Translator;
use simorgh_core::types::LangPair;
use simorgh_core::{Error, Result};

pub struct DeepLTranslator {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl DeepLTranslator {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        Ok(Self {
            client: crate::http_client(choice)?,
            base_url: choice.base_url.clone().unwrap_or_else(|| "https://api-free.deepl.com/v2".into()),
            api_key: crate::api_key(choice)?,
        })
    }
}

#[async_trait]
impl Translator for DeepLTranslator {
    fn id(&self) -> &'static str { "deepl" }
    fn display_name(&self) -> String { "DeepL (API)".into() }
    fn is_local(&self) -> bool { false }

    async fn translate(&self, text: &str, pair: &LangPair) -> Result<String> {
        let resp = self
            .client
            .post(format!("{}/translate", self.base_url))
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .json(&serde_json::json!({
                "text": [text],
                "source_lang": pair.source.0.to_uppercase(),
                "target_lang": pair.target.0.to_uppercase(),
            }))
            .send()
            .await
            .map_err(|e| err(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(err(format!("HTTP {}", resp.status())));
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| err(e.to_string()))?;
        Ok(body["translations"][0]["text"].as_str().unwrap_or_default().to_string())
    }
}

fn err(message: String) -> Error {
    Error::Engine { engine: "deepl".into(), message, fix: Some("Check your DeepL API key in Settings → Engines".into()) }
}

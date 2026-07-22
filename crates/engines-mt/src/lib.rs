//! Translation engines behind `simorgh_core::traits::Translator`.
//!
//! | Engine | Type | Notes |
//! |---|---|---|
//! | `openai-compat` | API/local | chat completions; any base URL (OpenAI, llama.cpp, Ollama, LM Studio) |
//! | `deepl` | API | REST v2 |
//! | `google` | API | Translation v2 REST |
//! | `nllb-local` | local | bundled NLLB via ONNX Runtime (feature `nllb`) |
//! | `mock` | local | echo translator for tests/CI |
//!
//! The `openai-compat` adapter doubles as the "local LLM" path: point the
//! base URL at llama.cpp / Ollama / LM Studio and it runs fully offline.

pub mod deepl;
pub mod google;
pub mod mock;
pub mod openai_compat;

use simorgh_core::config::EngineChoice;
use simorgh_core::traits::Translator;
use simorgh_core::{Error, Result};
use std::sync::Arc;

pub fn create(choice: &EngineChoice) -> Result<Arc<dyn Translator>> {
    match choice.engine.as_str() {
        "openai-compat" => Ok(Arc::new(openai_compat::OpenAiCompatTranslator::new(choice)?)),
        "deepl" => Ok(Arc::new(deepl::DeepLTranslator::new(choice)?)),
        "google" => Ok(Arc::new(google::GoogleTranslator::new(choice)?)),
        "mock" => Ok(Arc::new(mock::MockTranslator)),
        other => Err(Error::Engine {
            engine: other.into(),
            message: "unknown translator id".into(),
            fix: Some("Pick an engine in Settings → Engines → Translation".into()),
        }),
    }
}

pub(crate) fn http_client(choice: &EngineChoice) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Some(proxy) = &choice.proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy).map_err(|e| Error::Engine {
            engine: choice.engine.clone(),
            message: format!("invalid proxy: {e}"),
            fix: Some("Check the proxy URL in engine settings".into()),
        })?);
    }
    builder.build().map_err(|e| Error::Other(e.into()))
}

pub(crate) fn api_key(choice: &EngineChoice) -> Result<String> {
    let entry_name = choice.api_key_ref.as_deref().unwrap_or(&choice.engine);
    keyring::Entry::new("simorgh", entry_name)
        .and_then(|e| e.get_password())
        .map_err(|_| Error::Engine {
            engine: choice.engine.clone(),
            message: "API key not found in system keychain".into(),
            fix: Some("Enter your API key in Settings → Engines".into()),
        })
}

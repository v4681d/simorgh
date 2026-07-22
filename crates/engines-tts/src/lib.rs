//! TTS engine implementations behind `simorgh_core::traits::TtsEngine`.
//!
//! | Engine | Type | Streaming | Notes |
//! |---|---|---|---|
//! | `piper` | local | chunked | default fast local voice (subprocess) |
//! | `kokoro` | local | chunked | ONNX Runtime (feature `kokoro`) |
//! | `openai-tts` | API | yes | any OpenAI-compatible /audio/speech endpoint |
//! | `elevenlabs` | API | yes (HTTP stream) | |
//! | `mock` | local | yes | silence generator for tests/CI |
//!
//! IMPORTANT: every engine must set `AudioFrame::self_generated = true` on
//! output frames — that flag drives the anti-feedback gate in Pipeline A.

pub mod elevenlabs;
pub mod mock;
pub mod openai_tts;
pub mod piper;

use simorgh_core::config::EngineChoice;
use simorgh_core::traits::TtsEngine;
use simorgh_core::{Error, Result};
use std::sync::Arc;

pub fn create(choice: &EngineChoice) -> Result<Arc<dyn TtsEngine>> {
    match choice.engine.as_str() {
        "piper" => Ok(Arc::new(piper::PiperTts::new(choice)?)),
        "openai-tts" => Ok(Arc::new(openai_tts::OpenAiTts::new(choice)?)),
        "elevenlabs" => Ok(Arc::new(elevenlabs::ElevenLabsTts::new(choice)?)),
        "mock" => Ok(Arc::new(mock::MockTts)),
        other => Err(Error::Engine {
            engine: other.into(),
            message: "unknown TTS engine id".into(),
            fix: Some("Pick an engine in Settings → Engines → Text-to-speech".into()),
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

pub fn models_dir() -> std::path::PathBuf {
    directories::ProjectDirs::from("app", "simorgh", "simorgh")
        .map(|d| d.data_dir().join("models"))
        .unwrap_or_else(|| std::path::PathBuf::from("models"))
}

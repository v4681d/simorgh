//! STT engine implementations behind `simorgh_core::traits::SttEngine`.
//!
//! | Engine | Type | Streaming | Notes |
//! |---|---|---|---|
//! | `whisper-local` | local | pseudo (chunked) | whisper.cpp via whisper-rs, GPU features: cuda/metal/vulkan |
//! | `openai-stt` | API | chunked | any OpenAI-compatible /audio/transcriptions endpoint |
//! | `deepgram` | API | true WebSocket | interim + final results, endpointing flags |
//! | `custom` | API | WebSocket/HTTP | bring-your-own endpoint adapter |
//! | `mock` | local | yes | deterministic engine for tests/CI |

pub mod custom_ws;
pub mod deepgram;
pub mod mock;
pub mod openai;
#[cfg(feature = "whisper")]
pub mod whisper_local;

use simorgh_core::config::EngineChoice;
use simorgh_core::traits::SttEngine;
use simorgh_core::{Error, Result};
use std::sync::Arc;

/// Resolve an engine id from config into a live engine instance.
pub fn create(choice: &EngineChoice) -> Result<Arc<dyn SttEngine>> {
    match choice.engine.as_str() {
        #[cfg(feature = "whisper")]
        "whisper-local" => Ok(Arc::new(whisper_local::WhisperLocal::new(choice)?)),
        "openai-stt" => Ok(Arc::new(openai::OpenAiStt::new(choice)?)),
        "deepgram" => Ok(Arc::new(deepgram::DeepgramStt::new(choice)?)),
        "custom" => Ok(Arc::new(custom_ws::CustomEndpointStt::new(choice)?)),
        "mock" => Ok(Arc::new(mock::MockStt::default())),
        other => Err(Error::Engine {
            engine: other.into(),
            message: "unknown STT engine id".into(),
            fix: Some("Pick an engine in Settings → Engines → Speech-to-text".into()),
        }),
    }
}

/// Shared helper: build a reqwest client honoring base_url/proxy settings.
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

/// Shared helper: API key from the OS keychain (never plaintext config).
pub(crate) fn api_key(choice: &EngineChoice) -> Result<String> {
    let entry_name = choice.api_key_ref.as_deref().unwrap_or(&choice.engine);
    let entry = keyring_entry(entry_name)?;
    entry.get_password().map_err(|_| Error::Engine {
        engine: choice.engine.clone(),
        message: "API key not found in system keychain".into(),
        fix: Some("Enter your API key in Settings → Engines".into()),
    })
}

fn keyring_entry(name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new("simorgh", name).map_err(|e| Error::Other(e.into()))
}

/// Where the model manager stores downloaded model files.
pub fn models_dir() -> std::path::PathBuf {
    directories::ProjectDirs::from("app", "simorgh", "simorgh")
        .map(|d| d.data_dir().join("models"))
        .unwrap_or_else(|| std::path::PathBuf::from("models"))
}

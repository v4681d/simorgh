//! OpenAI-compatible TTS adapter (`/v1/audio/speech`) with streaming response
//! body -> AudioFrame conversion. Base URL configurable; works with OpenAI or
//! any compatible self-hosted server.

use async_trait::async_trait;
use futures::StreamExt;
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::TtsEngine;
use simorgh_core::types::{AudioFrame, Lang};
use simorgh_core::{Error, Result};
use std::time::Instant;
use tokio::sync::mpsc;

const PCM_RATE: u32 = 24_000;

pub struct OpenAiTts {
    client: reqwest::Client,
    base_url: String,
    model: String,
    voice: String,
    api_key: String,
}

impl OpenAiTts {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        // model field format: "tts-1:alloy" (model:voice) or just model.
        let raw = choice.model.clone().unwrap_or_else(|| "tts-1:alloy".into());
        let (model, voice) = match raw.split_once(':') {
            Some((m, v)) => (m.to_string(), v.to_string()),
            None => (raw, "alloy".to_string()),
        };
        Ok(Self {
            client: crate::http_client(choice)?,
            base_url: choice
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
            model,
            voice,
            api_key: crate::api_key(choice)?,
        })
    }
}

#[async_trait]
impl TtsEngine for OpenAiTts {
    fn id(&self) -> &'static str {
        "openai-tts"
    }
    fn display_name(&self) -> String {
        format!("{} / {} (API)", self.model, self.voice)
    }
    fn is_local(&self) -> bool {
        false
    }

    async fn synthesize(
        &self,
        text: &str,
        _lang: Lang,
        audio_tx: mpsc::Sender<AudioFrame>,
    ) -> Result<()> {
        let resp = self
            .client
            .post(format!("{}/audio/speech", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": self.model,
                "voice": self.voice,
                "input": text,
                "response_format": "pcm", // s16le @ 24 kHz
            }))
            .send()
            .await
            .map_err(|e| err(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(err(format!("HTTP {}", resp.status())));
        }
        let mut stream = resp.bytes_stream();
        let mut pending: Vec<u8> = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| err(e.to_string()))?;
            pending.extend_from_slice(&chunk);
            // Emit complete 100 ms frames as they arrive (streaming playback).
            let frame_bytes = (PCM_RATE as usize / 10) * 2;
            while pending.len() >= frame_bytes {
                let take: Vec<u8> = pending.drain(..frame_bytes).collect();
                let samples = bytes_to_f32(&take);
                if audio_tx.send(mk_frame(samples)).await.is_err() {
                    return Ok(());
                }
            }
        }
        if pending.len() >= 2 {
            let samples = bytes_to_f32(&pending);
            let _ = audio_tx.send(mk_frame(samples)).await;
        }
        Ok(())
    }
}

fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect()
}

fn mk_frame(samples: Vec<f32>) -> AudioFrame {
    AudioFrame {
        samples,
        sample_rate: PCM_RATE,
        captured_at: Instant::now(),
        self_generated: true,
    }
}

fn err(message: String) -> Error {
    Error::Engine {
        engine: "openai-tts".into(),
        message,
        fix: Some("Check base URL and API key in Settings → Engines".into()),
    }
}

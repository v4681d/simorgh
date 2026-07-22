//! ElevenLabs streaming TTS adapter (HTTP streaming endpoint, PCM output).

use async_trait::async_trait;
use futures::StreamExt;
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::TtsEngine;
use simorgh_core::types::{AudioFrame, Lang};
use simorgh_core::{Error, Result};
use std::time::Instant;
use tokio::sync::mpsc;

const PCM_RATE: u32 = 16_000;

pub struct ElevenLabsTts {
    client: reqwest::Client,
    base_url: String,
    voice_id: String,
    api_key: String,
}

impl ElevenLabsTts {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        Ok(Self {
            client: crate::http_client(choice)?,
            base_url: choice
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.elevenlabs.io/v1".into()),
            voice_id: choice
                .model
                .clone()
                .unwrap_or_else(|| "21m00Tcm4TlvDq8ikWAM".into()),
            api_key: crate::api_key(choice)?,
        })
    }
}

#[async_trait]
impl TtsEngine for ElevenLabsTts {
    fn id(&self) -> &'static str {
        "elevenlabs"
    }
    fn display_name(&self) -> String {
        "ElevenLabs (API, streaming)".into()
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
        let url = format!(
            "{}/text-to-speech/{}/stream?output_format=pcm_16000",
            self.base_url, self.voice_id
        );
        let resp = self
            .client
            .post(url)
            .header("xi-api-key", &self.api_key)
            .json(&serde_json::json!({
                "text": text,
                "model_id": "eleven_turbo_v2_5",
            }))
            .send()
            .await
            .map_err(|e| err(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(err(format!("HTTP {}", resp.status())));
        }
        let mut stream = resp.bytes_stream();
        let mut pending: Vec<u8> = Vec::new();
        let frame_bytes = (PCM_RATE as usize / 10) * 2; // 100 ms
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| err(e.to_string()))?;
            pending.extend_from_slice(&chunk);
            while pending.len() >= frame_bytes {
                let take: Vec<u8> = pending.drain(..frame_bytes).collect();
                let samples: Vec<f32> = take
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
                    .collect();
                let frame = AudioFrame {
                    samples,
                    sample_rate: PCM_RATE,
                    captured_at: Instant::now(),
                    self_generated: true,
                };
                if audio_tx.send(frame).await.is_err() {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

fn err(message: String) -> Error {
    Error::Engine {
        engine: "elevenlabs".into(),
        message,
        fix: Some("Check your ElevenLabs API key in Settings → Engines".into()),
    }
}

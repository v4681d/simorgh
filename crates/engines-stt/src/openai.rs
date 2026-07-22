//! OpenAI-compatible STT adapter (`/v1/audio/transcriptions`).
//! Works with OpenAI, Groq, local whisper.cpp servers, etc. — base URL is
//! always configurable, never hardcoded. Not a true streaming protocol, so we
//! send VAD-bounded chunks and emit each response as a final segment.

use async_trait::async_trait;
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::{SttEngine, SttSession};
use simorgh_core::types::{AudioFrame, Lang, TranscriptSegment};
use simorgh_core::{Error, Result};
use tokio::sync::mpsc;

const CHUNK_SECS: f64 = 5.0;

pub struct OpenAiStt {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: String,
}

impl OpenAiStt {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        Ok(Self {
            client: crate::http_client(choice)?,
            base_url: choice.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".into()),
            model: choice.model.clone().unwrap_or_else(|| "whisper-1".into()),
            api_key: crate::api_key(choice)?,
        })
    }

    async fn transcribe_chunk(&self, samples: &[f32], rate: u32, lang: &Lang) -> Result<String> {
        let wav = pcm_to_wav(samples, rate);
        let part = reqwest::multipart::Part::bytes(wav)
            .file_name("chunk.wav")
            .mime_str("audio/wav")
            .map_err(|e| Error::Other(e.into()))?;
        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("language", lang.0.clone());
        let resp = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| engine_err(format!("request failed: {e}")))?;
        if !resp.status().is_success() {
            return Err(engine_err(format!("HTTP {}", resp.status())));
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| engine_err(e.to_string()))?;
        Ok(body["text"].as_str().unwrap_or_default().to_string())
    }
}

fn engine_err(message: String) -> Error {
    Error::Engine { engine: "openai-stt".into(), message, fix: Some("Check base URL and API key in Settings → Engines".into()) }
}

#[async_trait]
impl SttEngine for OpenAiStt {
    fn id(&self) -> &'static str { "openai-stt" }
    fn display_name(&self) -> String { format!("{} (API)", self.model) }
    fn is_local(&self) -> bool { false }

    async fn start_session(
        &self,
        lang: Lang,
        mut audio_rx: mpsc::Receiver<AudioFrame>,
        segments_tx: mpsc::Sender<TranscriptSegment>,
    ) -> Result<SttSession> {
        let this = Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            api_key: self.api_key.clone(),
        };
        let handle = tokio::spawn(async move {
            let mut buf: Vec<f32> = Vec::new();
            let mut rate = 16_000u32;
            let mut t = 0.0f64;
            while let Some(frame) = audio_rx.recv().await {
                rate = frame.sample_rate;
                buf.extend_from_slice(&frame.samples);
                let dur = buf.len() as f64 / rate as f64;
                if dur >= CHUNK_SECS {
                    let text = this.transcribe_chunk(&buf, rate, &lang).await?;
                    t += dur;
                    buf.clear();
                    if !text.trim().is_empty() {
                        let seg = TranscriptSegment {
                            id: uuid::Uuid::new_v4(), text, lang: lang.clone(),
                            is_final: true, t_start: t - dur, t_end: t,
                        };
                        if segments_tx.send(seg).await.is_err() { break; }
                    }
                }
            }
            // Flush remainder on close.
            if buf.len() as f64 / rate as f64 > 0.5 {
                let text = this.transcribe_chunk(&buf, rate, &lang).await?;
                if !text.trim().is_empty() {
                    let _ = segments_tx.send(TranscriptSegment {
                        id: uuid::Uuid::new_v4(), text, lang: lang.clone(),
                        is_final: true, t_start: t, t_end: t,
                    }).await;
                }
            }
            Ok(())
        });
        Ok(SttSession { handle })
    }
}

/// Minimal 16-bit PCM WAV encoder (mono).
pub fn pcm_to_wav(samples: &[f32], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn wav_header_is_valid() {
        let wav = super::pcm_to_wav(&[0.0; 160], 16_000);
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + 320);
    }
}

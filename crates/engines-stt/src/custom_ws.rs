//! Generic "custom endpoint" STT adapter: point Simorgh at any WebSocket or
//! HTTP endpoint that speaks a simple JSON protocol. Documented in
//! docs/adding-engines.md so self-hosters can bridge anything.
//!
//! WebSocket protocol (ws:// or wss:// base URL):
//!   client -> binary frames: 16 kHz mono s16le PCM
//!   server -> {"text": "...", "final": bool}
//! HTTP protocol (http:// or https:// base URL):
//!   POST <base>/transcribe with WAV body -> {"text": "..."}

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::{SttEngine, SttSession};
use simorgh_core::types::{AudioFrame, Lang, TranscriptSegment};
use simorgh_core::{Error, Result};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

pub struct CustomEndpointStt {
    base_url: String,
}

impl CustomEndpointStt {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        let base_url = choice.base_url.clone().ok_or_else(|| Error::Engine {
            engine: "custom".into(),
            message: "custom endpoint requires a base URL".into(),
            fix: Some("Set the endpoint URL in Settings → Engines".into()),
        })?;
        Ok(Self { base_url })
    }
}

#[async_trait]
impl SttEngine for CustomEndpointStt {
    fn id(&self) -> &'static str { "custom" }
    fn display_name(&self) -> String { format!("Custom endpoint ({})", self.base_url) }
    fn is_local(&self) -> bool { false }

    async fn start_session(
        &self,
        lang: Lang,
        mut audio_rx: mpsc::Receiver<AudioFrame>,
        segments_tx: mpsc::Sender<TranscriptSegment>,
    ) -> Result<SttSession> {
        if !self.base_url.starts_with("ws") {
            return Err(Error::Engine {
                engine: "custom".into(),
                message: "HTTP mode not yet wired; use a ws:// endpoint".into(),
                fix: Some("Use a WebSocket endpoint, or the openai-stt adapter for HTTP servers".into()),
            });
        }
        let (ws, _) = tokio_tungstenite::connect_async(&self.base_url)
            .await
            .map_err(|e| Error::Engine { engine: "custom".into(), message: e.to_string(), fix: None })?;
        let (mut sink, mut stream) = ws.split();

        let uplink = tokio::spawn(async move {
            while let Some(frame) = audio_rx.recv().await {
                let mut bytes = Vec::with_capacity(frame.samples.len() * 2);
                for s in &frame.samples {
                    bytes.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
                }
                if sink.send(Message::Binary(bytes)).await.is_err() { break; }
            }
        });

        let handle = tokio::spawn(async move {
            let mut seg_id = uuid::Uuid::new_v4();
            while let Some(Ok(Message::Text(text))) = stream.next().await {
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
                let Some(t) = v["text"].as_str().filter(|t| !t.is_empty()) else { continue };
                let is_final = v["final"].as_bool().unwrap_or(false);
                let seg = TranscriptSegment {
                    id: seg_id, text: t.to_string(), lang: lang.clone(),
                    is_final, t_start: 0.0, t_end: 0.0,
                };
                if segments_tx.send(seg).await.is_err() { break; }
                if is_final { seg_id = uuid::Uuid::new_v4(); }
            }
            uplink.abort();
            Ok(())
        });
        Ok(SttSession { handle })
    }
}

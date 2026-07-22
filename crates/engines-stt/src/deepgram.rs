//! Deepgram streaming WebSocket adapter — true streaming STT with interim
//! results and endpointing. We map `is_final`/`speech_final` onto our
//! partial/final segment model; `speech_final` doubles as an endpoint signal
//! the clause-commit policy prefers over home-grown boundary detection.

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::{SttEngine, SttSession};
use simorgh_core::types::{AudioFrame, Lang, TranscriptSegment};
use simorgh_core::{Error, Result};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

pub struct DeepgramStt {
    base_url: String,
    model: String,
    api_key: String,
}

impl DeepgramStt {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        Ok(Self {
            base_url: choice
                .base_url
                .clone()
                .unwrap_or_else(|| "wss://api.deepgram.com/v1/listen".into()),
            model: choice.model.clone().unwrap_or_else(|| "nova-2".into()),
            api_key: crate::api_key(choice)?,
        })
    }
}

#[async_trait]
impl SttEngine for DeepgramStt {
    fn id(&self) -> &'static str {
        "deepgram"
    }
    fn display_name(&self) -> String {
        format!("Deepgram {} (API, streaming)", self.model)
    }
    fn is_local(&self) -> bool {
        false
    }

    async fn start_session(
        &self,
        lang: Lang,
        mut audio_rx: mpsc::Receiver<AudioFrame>,
        segments_tx: mpsc::Sender<TranscriptSegment>,
    ) -> Result<SttSession> {
        let url = format!(
            "{}?model={}&language={}&interim_results=true&endpointing=300&encoding=linear16&sample_rate=16000&channels=1",
            self.base_url, self.model, lang.0
        );
        let mut request = url.into_client_request().map_err(to_err)?;
        request.headers_mut().insert(
            "Authorization",
            format!("Token {}", self.api_key)
                .parse()
                .map_err(|_| to_err_s("bad api key header"))?,
        );
        let (ws, _) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(to_err)?;
        let (mut sink, mut stream) = ws.split();

        // Uplink: f32 -> i16 LE frames.
        let uplink = tokio::spawn(async move {
            while let Some(frame) = audio_rx.recv().await {
                let mut bytes = Vec::with_capacity(frame.samples.len() * 2);
                for s in &frame.samples {
                    bytes.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
                }
                if sink.send(Message::Binary(bytes)).await.is_err() {
                    break;
                }
            }
            let _ = sink
                .send(Message::Text("{\"type\":\"CloseStream\"}".into()))
                .await;
        });

        // Downlink: JSON results -> TranscriptSegment.
        let handle = tokio::spawn(async move {
            let mut seg_id = uuid::Uuid::new_v4();
            while let Some(msg) = stream.next().await {
                let Ok(Message::Text(text)) = msg else {
                    continue;
                };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
                    continue;
                };
                let transcript = v["channel"]["alternatives"][0]["transcript"]
                    .as_str()
                    .unwrap_or("");
                if transcript.is_empty() {
                    continue;
                }
                let is_final = v["is_final"].as_bool().unwrap_or(false);
                let speech_final = v["speech_final"].as_bool().unwrap_or(false);
                let seg = TranscriptSegment {
                    id: seg_id,
                    text: transcript.to_string(),
                    lang: lang.clone(),
                    is_final: is_final && speech_final,
                    t_start: v["start"].as_f64().unwrap_or(0.0),
                    t_end: v["start"].as_f64().unwrap_or(0.0)
                        + v["duration"].as_f64().unwrap_or(0.0),
                };
                let ended = seg.is_final;
                if segments_tx.send(seg).await.is_err() {
                    break;
                }
                if ended {
                    seg_id = uuid::Uuid::new_v4();
                }
            }
            uplink.abort();
            Ok(())
        });
        Ok(SttSession { handle })
    }
}

use tokio_tungstenite::tungstenite::client::IntoClientRequest;

fn to_err<E: std::fmt::Display>(e: E) -> Error {
    Error::Engine {
        engine: "deepgram".into(),
        message: e.to_string(),
        fix: Some("Check your Deepgram API key and network".into()),
    }
}
fn to_err_s(s: &str) -> Error {
    to_err(s.to_string())
}

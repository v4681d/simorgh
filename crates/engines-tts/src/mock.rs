//! Mock TTS — emits 500 ms of silence per request; used by tests/CI to prove
//! pipeline routing and the anti-feedback gate without any model.

use async_trait::async_trait;
use simorgh_core::traits::TtsEngine;
use simorgh_core::types::{AudioFrame, Lang};
use simorgh_core::Result;
use std::time::Instant;
use tokio::sync::mpsc;

pub struct MockTts;

#[async_trait]
impl TtsEngine for MockTts {
    fn id(&self) -> &'static str {
        "mock"
    }
    fn display_name(&self) -> String {
        "Mock TTS (testing)".into()
    }
    fn is_local(&self) -> bool {
        true
    }

    async fn synthesize(
        &self,
        _text: &str,
        _lang: Lang,
        audio_tx: mpsc::Sender<AudioFrame>,
    ) -> Result<()> {
        for _ in 0..5 {
            let frame = AudioFrame {
                samples: vec![0.0; 1600], // 100 ms @ 16 kHz
                sample_rate: 16_000,
                captured_at: Instant::now(),
                self_generated: true,
            };
            if audio_tx.send(frame).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}

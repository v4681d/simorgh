//! Deterministic mock STT for tests and CI: emits a growing partial for each
//! burst of audio, then a final segment after a quiet gap. No network, no
//! models — integration tests drive it with recorded fixtures.

use async_trait::async_trait;
use simorgh_core::traits::{SttEngine, SttSession};
use simorgh_core::types::{AudioFrame, Lang, TranscriptSegment};
use simorgh_core::Result;
use tokio::sync::mpsc;

#[derive(Default)]
pub struct MockStt {
    /// Words "recognized" per audio burst, cycled in order.
    pub script: Vec<String>,
}

#[async_trait]
impl SttEngine for MockStt {
    fn id(&self) -> &'static str { "mock" }
    fn display_name(&self) -> String { "Mock STT (testing)".into() }
    fn is_local(&self) -> bool { true }

    async fn start_session(
        &self,
        lang: Lang,
        mut audio_rx: mpsc::Receiver<AudioFrame>,
        segments_tx: mpsc::Sender<TranscriptSegment>,
    ) -> Result<SttSession> {
        let script = if self.script.is_empty() {
            vec!["hello".into(), "hello world".into(), "hello world again".into()]
        } else {
            self.script.clone()
        };
        let handle = tokio::spawn(async move {
            let id = uuid::Uuid::new_v4();
            let mut i = 0usize;
            let mut t = 0.0f64;
            while let Some(frame) = audio_rx.recv().await {
                let dur = frame.samples.len() as f64 / frame.sample_rate as f64;
                t += dur;
                let is_last = i + 1 >= script.len();
                let seg = TranscriptSegment {
                    id,
                    text: script[i.min(script.len() - 1)].clone(),
                    lang: lang.clone(),
                    is_final: is_last,
                    t_start: 0.0,
                    t_end: t,
                };
                if segments_tx.send(seg).await.is_err() {
                    break;
                }
                if !is_last { i += 1; }
            }
            Ok(())
        });
        Ok(SttSession { handle })
    }
}

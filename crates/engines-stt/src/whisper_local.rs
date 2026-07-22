//! Local whisper.cpp engine via `whisper-rs` (feature `whisper`).
//!
//! Streaming strategy: whisper is not natively streaming, so we run a rolling
//! window — accumulate audio, re-decode the tail every ~500 ms and emit the
//! hypothesis as a partial; when VAD upstream stops feeding audio (segment
//! gap), the last decode is emitted as final. Models (tiny..large-v3,
//! quantized variants) are provisioned by the app's model manager; GPU accel
//! is selected via the cuda/metal/vulkan cargo features.

use async_trait::async_trait;
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::{SttEngine, SttSession};
use simorgh_core::types::{AudioFrame, Lang, TranscriptSegment};
use simorgh_core::{Error, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

const DECODE_INTERVAL_MS: u64 = 500;
const MAX_WINDOW_SECS: usize = 30;

pub struct WhisperLocal {
    model_path: PathBuf,
    model_name: String,
}

impl WhisperLocal {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        let model_name = choice.model.clone().unwrap_or_else(|| "large-v3".into());
        let model_path = crate::models_dir().join(format!("ggml-{model_name}.bin"));
        if !model_path.exists() {
            return Err(Error::ModelMissing {
                model: format!("whisper {model_name}"),
                fix: "Open Models and download it (Settings → Models)".into(),
            });
        }
        Ok(Self {
            model_path,
            model_name,
        })
    }
}

#[async_trait]
impl SttEngine for WhisperLocal {
    fn id(&self) -> &'static str {
        "whisper-local"
    }
    fn display_name(&self) -> String {
        format!("Whisper {} (local)", self.model_name)
    }
    fn is_local(&self) -> bool {
        true
    }

    async fn start_session(
        &self,
        lang: Lang,
        mut audio_rx: mpsc::Receiver<AudioFrame>,
        segments_tx: mpsc::Sender<TranscriptSegment>,
    ) -> Result<SttSession> {
        let ctx = whisper_rs::WhisperContext::new_with_params(
            self.model_path.to_str().unwrap(),
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| Error::Engine {
            engine: "whisper-local".into(),
            message: format!("failed to load model: {e}"),
            fix: Some("Re-download the model in Settings → Models".into()),
        })?;
        let ctx = Arc::new(ctx);
        info!(model = %self.model_name, "whisper session starting");

        let handle = tokio::spawn(async move {
            let mut window: Vec<f32> = Vec::new();
            let mut ticker =
                tokio::time::interval(std::time::Duration::from_millis(DECODE_INTERVAL_MS));
            let seg_id = uuid::Uuid::new_v4();
            let mut last_text = String::new();
            loop {
                tokio::select! {
                    frame = audio_rx.recv() => {
                        match frame {
                            Some(f) => {
                                window.extend_from_slice(&f.samples);
                                let max = MAX_WINDOW_SECS * f.sample_rate as usize;
                                if window.len() > max {
                                    let cut = window.len() - max;
                                    window.drain(..cut);
                                }
                            }
                            None => {
                                // Upstream closed: flush the final hypothesis.
                                if !last_text.is_empty() {
                                    let _ = segments_tx.send(TranscriptSegment {
                                        id: seg_id, text: last_text.clone(),
                                        lang: lang.clone(), is_final: true,
                                        t_start: 0.0, t_end: 0.0,
                                    }).await;
                                }
                                return Ok(());
                            }
                        }
                    }
                    _ = ticker.tick() => {
                        if window.len() < 8000 { continue; } // need >= 0.5 s
                        let ctx = ctx.clone();
                        let samples = window.clone();
                        let lang_code = lang.0.clone();
                        // whisper decode is CPU/GPU heavy: keep it off the runtime.
                        let text = tokio::task::spawn_blocking(move || {
                            let mut state = ctx.create_state().map_err(|e| anyhow::anyhow!("{e}"))?;
                            let mut params = whisper_rs::FullParams::new(
                                whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
                            params.set_language(Some(&lang_code));
                            params.set_no_timestamps(true);
                            params.set_single_segment(true);
                            state.full(params, &samples).map_err(|e| anyhow::anyhow!("{e}"))?;
                            let n = state.full_n_segments().map_err(|e| anyhow::anyhow!("{e}"))?;
                            let mut text = String::new();
                            for i in 0..n {
                                text.push_str(&state.full_get_segment_text(i).unwrap_or_default());
                            }
                            Ok::<String, anyhow::Error>(text.trim().to_string())
                        }).await.map_err(|e| Error::Other(e.into()))??;
                        if !text.is_empty() && text != last_text {
                            last_text = text.clone();
                            if segments_tx.send(TranscriptSegment {
                                id: seg_id, text, lang: lang.clone(), is_final: false,
                                t_start: 0.0, t_end: 0.0,
                            }).await.is_err() { return Ok(()); }
                        }
                    }
                }
            }
        });
        Ok(SttSession { handle })
    }
}

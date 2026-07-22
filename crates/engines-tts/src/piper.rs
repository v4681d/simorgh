//! Piper TTS as a subprocess — the default fast local voice. Piper reads text
//! on stdin and writes raw s16le PCM on stdout, which we stream out in ~100 ms
//! frames so playback (and the virtual mic) starts before synthesis finishes.
//! Voice models (.onnx + .json) are provisioned by the model manager.

use async_trait::async_trait;
use simorgh_core::config::EngineChoice;
use simorgh_core::traits::TtsEngine;
use simorgh_core::types::{AudioFrame, Lang};
use simorgh_core::{Error, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

const PIPER_RATE: u32 = 22_050;
const FRAME_SAMPLES: usize = 2_205; // 100 ms

pub struct PiperTts {
    voice_path: PathBuf,
    voice_name: String,
}

impl PiperTts {
    pub fn new(choice: &EngineChoice) -> Result<Self> {
        let voice_name = choice.model.clone().unwrap_or_else(|| "en_US-lessac-medium".into());
        let voice_path = crate::models_dir().join(format!("{voice_name}.onnx"));
        if !voice_path.exists() {
            return Err(Error::ModelMissing {
                model: format!("piper voice {voice_name}"),
                fix: "Open Models and download it (Settings → Models)".into(),
            });
        }
        Ok(Self { voice_path, voice_name })
    }
}

#[async_trait]
impl TtsEngine for PiperTts {
    fn id(&self) -> &'static str { "piper" }
    fn display_name(&self) -> String { format!("Piper {} (local)", self.voice_name) }
    fn is_local(&self) -> bool { true }

    async fn synthesize(
        &self,
        text: &str,
        _lang: Lang,
        audio_tx: mpsc::Sender<AudioFrame>,
    ) -> Result<()> {
        let mut child = tokio::process::Command::new("piper")
            .arg("--model").arg(&self.voice_path)
            .arg("--output-raw")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Engine {
                engine: "piper".into(),
                message: format!("failed to start piper: {e}"),
                fix: Some("Install piper-tts or choose another TTS engine".into()),
            })?;

        let mut stdin = child.stdin.take().expect("piped stdin");
        let text = text.to_string();
        tokio::spawn(async move {
            let _ = stdin.write_all(text.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });

        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut byte_buf = vec![0u8; FRAME_SAMPLES * 2];
        loop {
            let mut filled = 0;
            // Fill a whole frame (or hit EOF).
            while filled < byte_buf.len() {
                let n = stdout.read(&mut byte_buf[filled..]).await.map_err(Error::Io)?;
                if n == 0 { break; }
                filled += n;
            }
            if filled == 0 { break; }
            let samples: Vec<f32> = byte_buf[..filled]
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
                .collect();
            let frame = AudioFrame {
                samples,
                sample_rate: PIPER_RATE,
                captured_at: Instant::now(),
                self_generated: true, // anti-feedback gate depends on this
            };
            if audio_tx.send(frame).await.is_err() { break; }
            if filled < byte_buf.len() { break; } // EOF mid-frame
        }
        let _ = child.wait().await;
        Ok(())
    }
}

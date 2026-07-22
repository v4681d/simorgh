use serde::{Deserialize, Serialize};
use std::time::Instant;

/// BCP-47-ish language code ("en", "fa", ...). Generic so any pair works.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Lang(pub String);

impl Lang {
    pub fn en() -> Self { Lang("en".into()) }
    pub fn fa() -> Self { Lang("fa".into()) }
    /// Right-to-left script? Used by the UI for direction hints.
    pub fn is_rtl(&self) -> bool {
        matches!(self.0.as_str(), "fa" | "ar" | "he" | "ur" | "ps")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LangPair {
    pub source: Lang,
    pub target: Lang,
}

/// A frame of mono f32 PCM audio flowing between stages.
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    /// Capture timestamp — the anchor for end-to-end latency measurement.
    pub captured_at: Instant,
    /// True when this frame was produced by our own TTS (feedback-loop gate:
    /// Pipeline A must drop frames with this flag, see `pipeline::SelfAudioGate`).
    pub self_generated: bool,
}

/// Streaming STT output. `is_final == false` means a growing partial
/// hypothesis that may be rewritten; `true` means the segment is locked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: uuid::Uuid,
    pub text: String,
    pub lang: Lang,
    pub is_final: bool,
    /// Seconds from stream start.
    pub t_start: f64,
    pub t_end: f64,
}

/// Streaming translation output tied to a source segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationUpdate {
    pub segment_id: uuid::Uuid,
    pub source_text: String,
    pub translated_text: String,
    pub lang: LangPair,
    pub is_final: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineKind {
    /// Listen & subtitle (loopback -> STT -> MT -> overlay).
    Subtitle,
    /// Speak & translate (mic -> STT -> MT -> TTS -> virtual mic).
    Voice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Stopped,
    Starting,
    Running,
    Degraded,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Capture,
    Vad,
    Stt,
    Translate,
    Tts,
    Output,
}

/// Voice activity decision for one frame window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadEvent {
    SpeechStart,
    Speech,
    Silence,
    SpeechEnd,
}

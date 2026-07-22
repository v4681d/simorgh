use crate::types::*;
use crate::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Streaming speech-to-text. Feed audio frames in, receive partial/final
/// transcript segments out. Implementations must be cancel-safe: dropping the
/// session must release the model/socket.
#[async_trait]
pub trait SttEngine: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> String;
    /// True if the engine runs fully offline.
    fn is_local(&self) -> bool;
    /// Start a streaming session. Frames arrive on `audio_rx`; the engine
    /// pushes growing partials (`is_final = false`) and locked finals
    /// (`is_final = true`) into `segments_tx`.
    async fn start_session(
        &self,
        lang: Lang,
        audio_rx: mpsc::Receiver<AudioFrame>,
        segments_tx: mpsc::Sender<TranscriptSegment>,
    ) -> Result<SttSession>;
}

/// Handle to a live STT session; abort on drop.
pub struct SttSession {
    pub handle: tokio::task::JoinHandle<Result<()>>,
}

impl Drop for SttSession {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Text translation. `translate` must be cheap to cancel: Pipeline A
/// re-translates on every partial and aborts superseded requests.
#[async_trait]
pub trait Translator: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> String;
    fn is_local(&self) -> bool;
    async fn translate(&self, text: &str, pair: &LangPair) -> Result<String>;
    /// Optional LLM-assisted semantic boundary: return the longest prefix of
    /// `text` that is safe to translate/commit now, or None if unsupported.
    async fn safe_prefix(&self, _text: &str, _pair: &LangPair) -> Result<Option<String>> {
        Ok(None)
    }
}

/// Text-to-speech. Streaming synthesis when supported; otherwise emit one
/// final chunk. Output frames MUST set `AudioFrame::self_generated = true`.
#[async_trait]
pub trait TtsEngine: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> String;
    fn is_local(&self) -> bool;
    async fn synthesize(
        &self,
        text: &str,
        lang: Lang,
        audio_tx: mpsc::Sender<AudioFrame>,
    ) -> Result<()>;
}

/// Voice activity detection over fixed-size windows.
pub trait VadEngine: Send + Sync {
    fn id(&self) -> &'static str;
    /// Feed one window (e.g. 30 ms) of mono 16 kHz samples.
    fn process(&mut self, samples: &[f32]) -> VadEvent;
    fn set_sensitivity(&mut self, sensitivity: f32);
}

/// What the commit policy tells the pipeline to do after each STT update.
#[derive(Debug, Clone, PartialEq)]
pub enum CommitDecision {
    /// Do nothing yet (e.g. debounce window not elapsed).
    Hold,
    /// Translate this text and render/emit it as provisional (revisable).
    Provisional(String),
    /// Translate this text and lock it — never revise it afterwards.
    Commit(String),
}

/// Segmentation & commitment strategy (see docs/architecture.md §Commitment).
/// Implementations: `policy::RetranslationPolicy` (subtitles, eager+revisable)
/// and `policy::ClauseCommitPolicy` (voice, late+irreversible). Post-v1
/// strategies (wait-k, local agreement) plug in behind this same trait.
pub trait CommitPolicy: Send {
    fn id(&self) -> &'static str;
    /// Called on every STT update for the active segment.
    fn on_transcript(&mut self, seg: &TranscriptSegment, now_ms: u64) -> CommitDecision;
    /// Called when VAD reports end-of-speech for the active segment.
    fn on_speech_end(&mut self, seg: &TranscriptSegment) -> CommitDecision;
    /// Called on a timer tick so time-based triggers (max chunk window) fire
    /// even while the speaker keeps talking.
    fn on_tick(&mut self, now_ms: u64) -> CommitDecision;
}

//! Commitment policies. English is SVO, Persian is SOV: the Persian verb
//! arrives at the END of the clause, so fa->en must not commit early, while
//! subtitles (text) can eagerly re-translate and revise. Hence two policies.

use crate::traits::{CommitDecision, CommitPolicy};
use crate::types::TranscriptSegment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySettings {
    /// VAD silence that ends a phrase (ms). Lower = snappier, more cuts.
    pub silence_end_ms: u64,
    /// Forced-commit upper bound so rambling never stalls the pipeline (ms).
    pub max_chunk_ms: u64,
    /// Max rate for re-translation of partials (ms).
    pub retranslate_debounce_ms: u64,
}

impl Default for PolicySettings {
    fn default() -> Self {
        Self {
            silence_end_ms: 600,
            max_chunk_ms: 6_000,
            retranslate_debounce_ms: 1_000,
        }
    }
}

/// Pipeline A (subtitles): eager and revisable. Re-translate the whole
/// current segment on every update (debounced), render provisional at reduced
/// opacity, and lock exactly once on finalization. Never revise finals.
pub struct RetranslationPolicy {
    settings: PolicySettings,
    last_emit_ms: u64,
    pending: Option<String>,
}

impl RetranslationPolicy {
    pub fn new(settings: PolicySettings) -> Self {
        Self {
            settings,
            last_emit_ms: 0,
            pending: None,
        }
    }
}

impl CommitPolicy for RetranslationPolicy {
    fn id(&self) -> &'static str {
        "retranslation"
    }

    fn on_transcript(&mut self, seg: &TranscriptSegment, now_ms: u64) -> CommitDecision {
        if seg.is_final {
            self.pending = None;
            return CommitDecision::Commit(seg.text.clone());
        }
        if now_ms.saturating_sub(self.last_emit_ms) >= self.settings.retranslate_debounce_ms {
            self.last_emit_ms = now_ms;
            self.pending = None;
            CommitDecision::Provisional(seg.text.clone())
        } else {
            // Remember the newest hypothesis; flush it on the next tick.
            self.pending = Some(seg.text.clone());
            CommitDecision::Hold
        }
    }

    fn on_speech_end(&mut self, seg: &TranscriptSegment) -> CommitDecision {
        self.pending = None;
        CommitDecision::Commit(seg.text.clone())
    }

    fn on_tick(&mut self, now_ms: u64) -> CommitDecision {
        if let Some(text) = self.pending.take() {
            if now_ms.saturating_sub(self.last_emit_ms) >= self.settings.retranslate_debounce_ms {
                self.last_emit_ms = now_ms;
                return CommitDecision::Provisional(text);
            }
            self.pending = Some(text);
        }
        CommitDecision::Hold
    }
}

/// Pipeline B (voice): late and irreversible. Spoken audio cannot be un-said,
/// so commit only at clause boundaries — VAD silence >= threshold, or the max
/// chunk window as a forced backstop. Carry uncommitted remainder forward.
pub struct ClauseCommitPolicy {
    settings: PolicySettings,
    chunk_started_ms: Option<u64>,
    current_text: String,
    committed_len: usize,
}

impl ClauseCommitPolicy {
    pub fn new(settings: PolicySettings) -> Self {
        Self {
            settings,
            chunk_started_ms: None,
            current_text: String::new(),
            committed_len: 0,
        }
    }

    fn uncommitted(&self) -> &str {
        &self.current_text[self.committed_len.min(self.current_text.len())..]
    }

    fn commit_all(&mut self) -> CommitDecision {
        let chunk = self.uncommitted().trim().to_string();
        if chunk.is_empty() {
            return CommitDecision::Hold;
        }
        self.committed_len = self.current_text.len();
        self.chunk_started_ms = None;
        CommitDecision::Commit(chunk)
    }
}

impl CommitPolicy for ClauseCommitPolicy {
    fn id(&self) -> &'static str {
        "clause_commit"
    }

    fn on_transcript(&mut self, seg: &TranscriptSegment, now_ms: u64) -> CommitDecision {
        if self.chunk_started_ms.is_none() {
            self.chunk_started_ms = Some(now_ms);
        }
        self.current_text = seg.text.clone();
        if seg.is_final {
            // STT finalized (endpointing) — the clause verb has arrived.
            return self.commit_all();
        }
        CommitDecision::Hold
    }

    fn on_speech_end(&mut self, seg: &TranscriptSegment) -> CommitDecision {
        // VAD silence >= silence_end_ms fired upstream: end of phrase.
        self.current_text = seg.text.clone();
        self.commit_all()
    }

    fn on_tick(&mut self, now_ms: u64) -> CommitDecision {
        if let Some(start) = self.chunk_started_ms {
            if now_ms.saturating_sub(start) >= self.settings.max_chunk_ms
                && !self.uncommitted().trim().is_empty()
            {
                // Forced commit: never let a rambling sentence stall the pipe.
                return self.commit_all();
            }
        }
        CommitDecision::Hold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Lang;

    fn seg(text: &str, is_final: bool) -> TranscriptSegment {
        TranscriptSegment {
            id: uuid::Uuid::new_v4(),
            text: text.into(),
            lang: Lang::fa(),
            is_final,
            t_start: 0.0,
            t_end: 1.0,
        }
    }

    #[test]
    fn retranslation_debounces_partials_and_commits_finals() {
        let mut p = RetranslationPolicy::new(PolicySettings::default());
        assert!(matches!(
            p.on_transcript(&seg("hello", false), 1_000),
            CommitDecision::Provisional(_)
        ));
        assert!(matches!(
            p.on_transcript(&seg("hello wor", false), 1_200),
            CommitDecision::Hold
        ));
        assert!(matches!(p.on_tick(2_100), CommitDecision::Provisional(t) if t == "hello wor"));
        assert!(
            matches!(p.on_transcript(&seg("hello world", true), 2_200), CommitDecision::Commit(t) if t == "hello world")
        );
    }

    #[test]
    fn clause_commit_waits_for_boundary() {
        let mut p = ClauseCommitPolicy::new(PolicySettings::default());
        // Growing Persian clause: no commit until the boundary (verb arrives at end).
        assert!(matches!(
            p.on_transcript(&seg("من فردا", false), 0),
            CommitDecision::Hold
        ));
        assert!(matches!(
            p.on_transcript(&seg("من فردا به مدرسه", false), 500),
            CommitDecision::Hold
        ));
        let d = p.on_speech_end(&seg("من فردا به مدرسه می‌روم", false));
        assert!(matches!(d, CommitDecision::Commit(t) if t.ends_with("می‌روم")));
    }

    #[test]
    fn clause_commit_forces_after_max_window() {
        let mut p = ClauseCommitPolicy::new(PolicySettings::default());
        assert!(matches!(
            p.on_transcript(&seg("a very long ramble", false), 0),
            CommitDecision::Hold
        ));
        assert!(matches!(p.on_tick(5_999), CommitDecision::Hold));
        assert!(matches!(p.on_tick(6_000), CommitDecision::Commit(_)));
    }
}

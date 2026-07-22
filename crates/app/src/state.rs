//! Shared app state: config, running pipelines, meters, latency recorders,
//! and the transcript history buffer (exportable as TXT/SRT).

use simorgh_audio::LevelMeter;
use simorgh_core::config::AppConfig;
use simorgh_core::latency::LatencyRecorder;
use simorgh_core::pipeline::{PipelineHandle, SelfAudioGate};
use simorgh_core::types::{PipelineKind, TranslationUpdate};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // history write not yet wired into the pipeline
const HISTORY_CAP: usize = 2_000;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub subtitle_pipeline: Mutex<Option<PipelineHandle>>,
    pub voice_pipeline: Mutex<Option<PipelineHandle>>,
    /// Capture guards must outlive start_pipeline — holding them here keeps the
    /// cpal stream alive until the pipeline stops, else audio dies on return.
    pub subtitle_capture: Mutex<Option<simorgh_audio::capture::CaptureGuard>>,
    pub voice_capture: Mutex<Option<simorgh_audio::capture::CaptureGuard>>,
    pub gate: SelfAudioGate,
    pub latency: Arc<LatencyRecorder>,
    pub subtitle_meter: LevelMeter,
    pub voice_meter: LevelMeter,
    pub history: Mutex<VecDeque<HistoryEntry>>,
}

#[derive(Clone, serde::Serialize)]
pub struct HistoryEntry {
    pub at_ms: u64,
    pub kind: PipelineKind,
    pub source_text: String,
    pub translated_text: String,
}

impl AppState {
    pub fn load() -> Self {
        let config = AppConfig::load().unwrap_or_else(|e| {
            tracing::warn!(error = %e, "config load failed; using defaults");
            AppConfig::default()
        });
        Self {
            config: Mutex::new(config),
            subtitle_pipeline: Mutex::new(None),
            voice_pipeline: Mutex::new(None),
            subtitle_capture: Mutex::new(None),
            voice_capture: Mutex::new(None),
            gate: SelfAudioGate::default(),
            latency: Arc::new(LatencyRecorder::default()),
            subtitle_meter: LevelMeter::default(),
            voice_meter: LevelMeter::default(),
            history: Mutex::new(VecDeque::new()),
        }
    }

    #[allow(dead_code)] // history write not yet wired into the pipeline
    pub fn push_history(&self, kind: PipelineKind, u: &TranslationUpdate, at_ms: u64) {
        let mut h = self.history.lock().expect("history lock");
        if h.len() >= HISTORY_CAP {
            h.pop_front();
        }
        h.push_back(HistoryEntry {
            at_ms,
            kind,
            source_text: u.source_text.clone(),
            translated_text: u.translated_text.clone(),
        });
    }

    pub fn persist_overlay_bounds(&self, window: &tauri::Window) {
        let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) else {
            return;
        };
        let mut cfg = self.config.lock().expect("config lock");
        cfg.subtitle.x = Some(pos.x);
        cfg.subtitle.y = Some(pos.y);
        cfg.subtitle.width = Some(size.width);
        let _ = cfg.save();
    }
}

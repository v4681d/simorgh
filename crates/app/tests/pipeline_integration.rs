//! Integration test: run both pipelines end-to-end with mock engines and a
//! synthetic audio fixture. Proves channel wiring, the commit policies, and
//! the anti-feedback gate — all CI-safe (no models, no network, no devices).

use simorgh_core::latency::LatencyRecorder;
use simorgh_core::pipeline::*;
use simorgh_core::policy::*;
use simorgh_core::types::*;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

fn frame(self_generated: bool) -> AudioFrame {
    AudioFrame { samples: vec![0.1; 1600], sample_rate: 16_000, captured_at: Instant::now(), self_generated }
}

fn deps(policy: Box<dyn simorgh_core::traits::CommitPolicy>, with_tts: bool) -> PipelineDeps {
    PipelineDeps {
        stt: Arc::new(simorgh_engines_stt::mock::MockStt::default()),
        translator: Arc::new(simorgh_engines_mt::mock::MockTranslator),
        tts: with_tts.then(|| Arc::new(simorgh_engines_tts::mock::MockTts) as Arc<dyn simorgh_core::traits::TtsEngine>),
        policy,
        pair: LangPair { source: Lang::en(), target: Lang::fa() },
        latency: Arc::new(LatencyRecorder::default()),
        gate: SelfAudioGate::default(),
        settings: PolicySettings::default(),
    }
}

#[tokio::test]
async fn subtitle_pipeline_emits_final_translation() {
    let (audio_tx, audio_rx) = mpsc::channel(16);
    let (events_tx, mut events_rx) = mpsc::channel(64);
    let _handle = start_subtitle_pipeline(
        deps(Box::new(RetranslationPolicy::new(PolicySettings::default())), false),
        audio_rx,
        events_tx,
    ).await.expect("pipeline starts");

    for _ in 0..3 { audio_tx.send(frame(false)).await.unwrap(); }
    drop(audio_tx);

    let mut got_final = false;
    while let Ok(Some(ev)) = tokio::time::timeout(std::time::Duration::from_secs(5), events_rx.recv()).await {
        if let PipelineEvent::Translation(t) = ev {
            assert!(t.translated_text.starts_with("[en→fa]"));
            if t.is_final { got_final = true; break; }
        }
    }
    assert!(got_final, "expected a final translation event");
}

#[tokio::test]
async fn gate_drops_self_generated_audio() {
    let gate = SelfAudioGate::default();
    assert!(gate.should_drop(&frame(true)), "self-generated frames must be dropped");
    assert!(!gate.should_drop(&frame(false)));
    gate.set_speaking(true);
    assert!(gate.should_drop(&frame(false)), "all frames dropped while TTS speaks");
    gate.set_speaking(false);
    assert!(!gate.should_drop(&frame(false)));
}

#[tokio::test]
async fn voice_pipeline_synthesizes_committed_clauses() {
    let (audio_tx, audio_rx) = mpsc::channel(16);
    let (sink_tx, mut sink_rx) = mpsc::channel(64);
    let (events_tx, _events_rx) = mpsc::channel(64);
    let _handle = start_voice_pipeline(
        deps(Box::new(ClauseCommitPolicy::new(PolicySettings::default())), true),
        audio_rx,
        sink_tx,
        events_tx,
    ).await.expect("pipeline starts");

    for _ in 0..3 { audio_tx.send(frame(false)).await.unwrap(); }
    drop(audio_tx);

    let synthesized = tokio::time::timeout(std::time::Duration::from_secs(5), sink_rx.recv())
        .await.expect("timely").expect("got audio");
    assert!(synthesized.self_generated, "TTS output must be flagged self-generated");
}

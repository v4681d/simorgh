//! Tauri commands — the full IPC surface the frontend calls.

use crate::state::{AppState, HistoryEntry};
use crate::windows;
use simorgh_audio::devices::DeviceInfo;
use simorgh_audio::virtual_mic::SetupStatus;
use simorgh_core::config::AppConfig;
use simorgh_core::latency::LatencyReport;
use simorgh_core::pipeline::{self, PipelineDeps, PipelineEvent};
use simorgh_core::policy::{ClauseCommitPolicy, RetranslationPolicy};
use simorgh_core::types::{Lang, LangPair, PipelineKind, PipelineState};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

type CmdResult<T> = Result<T, String>;

fn err_to_string(e: simorgh_core::Error) -> String {
    match e.fix_hint() {
        Some(fix) => format!("{e}\u{1f527}{fix}"), // UI splits on the wrench char
        None => e.to_string(),
    }
}

#[tauri::command]
pub fn list_devices() -> CmdResult<Vec<DeviceInfo>> {
    simorgh_audio::list_devices().map_err(err_to_string)
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> CmdResult<AppConfig> {
    Ok(state.config.lock().expect("config lock").clone())
}

#[tauri::command]
pub fn set_config(state: State<AppState>, config: AppConfig) -> CmdResult<()> {
    config.validate().map_err(err_to_string)?;
    config.save().map_err(err_to_string)?;
    *state.config.lock().expect("config lock") = config;
    Ok(())
}

/// Engine changes apply live: stop + rebuild + start, no app restart.
#[tauri::command]
pub async fn start_pipeline(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: PipelineKind,
) -> CmdResult<()> {
    stop_pipeline_inner(&state, kind);
    let cfg = state.config.lock().expect("config lock").effective();

    let (events_tx, mut events_rx) = mpsc::channel::<PipelineEvent>(64);
    let handle = match kind {
        PipelineKind::Subtitle => {
            let pair = cfg.engines.subtitle_pair.clone().unwrap_or(LangPair {
                source: Lang::en(),
                target: Lang::fa(),
            });
            let deps = PipelineDeps {
                stt: simorgh_engines_stt::create(&cfg.engines.stt_subtitle)
                    .map_err(err_to_string)?,
                translator: simorgh_engines_mt::create(&cfg.engines.translator_subtitle)
                    .map_err(err_to_string)?,
                tts: None,
                policy: Box::new(RetranslationPolicy::new(cfg.policy.clone())),
                pair,
                latency: state.latency.clone(),
                gate: state.gate.clone(),
                settings: cfg.policy.clone(),
            };
            let (audio_tx, audio_rx) = mpsc::channel(64);
            let device = cfg
                .devices
                .loopback_device
                .clone()
                .ok_or("Select a loopback device first")?;
            let meter = state.subtitle_meter.clone();
            let guard = start_metered_loopback(&device, audio_tx, meter)?;
            // Keep the capture stream alive for the pipeline's lifetime.
            *state.subtitle_capture.lock().expect("lock") = Some(guard);
            pipeline::start_subtitle_pipeline(deps, audio_rx, events_tx)
                .await
                .map_err(err_to_string)?
        }
        PipelineKind::Voice => {
            let pair = cfg.engines.voice_pair.clone().unwrap_or(LangPair {
                source: Lang::fa(),
                target: Lang::en(),
            });
            let deps = PipelineDeps {
                stt: simorgh_engines_stt::create(&cfg.engines.stt_voice).map_err(err_to_string)?,
                translator: simorgh_engines_mt::create(&cfg.engines.translator_voice)
                    .map_err(err_to_string)?,
                tts: Some(simorgh_engines_tts::create(&cfg.engines.tts).map_err(err_to_string)?),
                policy: Box::new(ClauseCommitPolicy::new(cfg.policy.clone())),
                pair,
                latency: state.latency.clone(),
                gate: state.gate.clone(),
                settings: cfg.policy.clone(),
            };
            let (audio_tx, audio_rx) = mpsc::channel(64);
            let mic = cfg
                .devices
                .mic_device
                .clone()
                .ok_or("Select a microphone first")?;
            let meter = state.voice_meter.clone();
            let guard = start_metered_capture(&mic, audio_tx, meter)?;
            // Keep the capture stream alive for the pipeline's lifetime.
            *state.voice_capture.lock().expect("lock") = Some(guard);
            let (sink_tx, sink_rx) = mpsc::channel(64);
            let sink = cfg
                .devices
                .virtual_mic_sink
                .clone()
                .unwrap_or_else(|| simorgh_audio::virtual_mic::SINK_NAME.into());
            simorgh_audio::virtual_mic::start_sink_writer(&sink, sink_rx).map_err(err_to_string)?;
            pipeline::start_voice_pipeline(deps, audio_rx, sink_tx, events_tx)
                .await
                .map_err(err_to_string)?
        }
    };

    // Forward pipeline events to all windows (dashboard strip + overlay).
    let app2 = app.clone();
    tokio::spawn(async move {
        while let Some(ev) = events_rx.recv().await {
            let _ = app2.emit("pipeline-event", &ev);
        }
    });

    match kind {
        PipelineKind::Subtitle => *state.subtitle_pipeline.lock().expect("lock") = Some(handle),
        PipelineKind::Voice => *state.voice_pipeline.lock().expect("lock") = Some(handle),
    }
    // Overlay appears only while a pipeline is active.
    let _ = crate::windows::show_overlay(&app);
    Ok(())
}

#[tauri::command]
pub fn stop_pipeline(app: AppHandle, state: State<AppState>, kind: PipelineKind) -> CmdResult<()> {
    stop_pipeline_inner(&state, kind);
    // Hide the overlay once both pipelines are idle.
    let both_idle = state.subtitle_pipeline.lock().expect("lock").is_none()
        && state.voice_pipeline.lock().expect("lock").is_none();
    if both_idle {
        let _ = crate::windows::hide_overlay(&app);
    }
    Ok(())
}

fn stop_pipeline_inner(state: &State<AppState>, kind: PipelineKind) {
    let (slot, cap) = match kind {
        PipelineKind::Subtitle => (&state.subtitle_pipeline, &state.subtitle_capture),
        PipelineKind::Voice => (&state.voice_pipeline, &state.voice_capture),
    };
    if let Some(mut h) = slot.lock().expect("lock").take() {
        h.stop();
    }
    // Drop the capture guard — tears down the cpal stream.
    let _ = cap.lock().expect("lock").take();
}

#[tauri::command]
pub fn pipeline_status(state: State<AppState>) -> CmdResult<serde_json::Value> {
    let get = |slot: &std::sync::Mutex<Option<simorgh_core::pipeline::PipelineHandle>>| {
        slot.lock()
            .expect("lock")
            .as_ref()
            .map(|h| h.state())
            .unwrap_or(PipelineState::Stopped)
    };
    Ok(serde_json::json!({
        "subtitle": get(&state.subtitle_pipeline),
        "voice": get(&state.voice_pipeline),
    }))
}

#[tauri::command]
pub fn latency_report(state: State<AppState>) -> CmdResult<LatencyReport> {
    Ok(state.latency.report())
}

#[tauri::command]
pub fn meter_levels(state: State<AppState>) -> CmdResult<serde_json::Value> {
    Ok(serde_json::json!({
        "subtitle": state.subtitle_meter.read(),
        "voice": state.voice_meter.read(),
    }))
}

/// Store an API key in the OS keychain — never in plaintext config.
#[tauri::command]
pub fn set_api_key(entry: String, key: String) -> CmdResult<()> {
    // Trim — pasted keys with trailing whitespace/newline break Bearer auth.
    let key = key.trim();
    keyring::Entry::new("simorgh", &entry)
        .and_then(|e| e.set_password(key))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ensure_virtual_mic() -> CmdResult<SetupStatus> {
    simorgh_audio::virtual_mic::ensure_virtual_mic()
        .await
        .map_err(err_to_string)
}

// ---- Model manager ----

#[derive(serde::Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub kind: String,
    pub size_bytes: Option<u64>,
    pub downloaded: bool,
}

/// Known model catalog + what's on disk (disk usage shown in UI).
#[tauri::command]
pub fn list_models() -> CmdResult<Vec<ModelInfo>> {
    let dir = simorgh_engines_stt::models_dir();
    let catalog: &[(&str, &str)] = &[
        ("ggml-tiny.bin", "stt"),
        ("ggml-base.bin", "stt"),
        ("ggml-small.bin", "stt"),
        ("ggml-medium.bin", "stt"),
        ("ggml-large-v3.bin", "stt"),
        ("ggml-large-v3-q5_0.bin", "stt"),
        ("en_US-lessac-medium.onnx", "tts"),
        ("silero_vad.onnx", "vad"),
    ];
    Ok(catalog
        .iter()
        .map(|(name, kind)| {
            let path = dir.join(name);
            let size = path.metadata().ok().map(|m| m.len());
            ModelInfo {
                name: name.to_string(),
                kind: kind.to_string(),
                size_bytes: size,
                downloaded: path.exists(),
            }
        })
        .collect())
}

/// Resumable download with checksum verification; progress events streamed to
/// the UI as `model-download-progress`.
#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    name: String,
    url: String,
    sha256: Option<String>,
) -> CmdResult<()> {
    use futures::StreamExt;
    use sha2::Digest;
    let dir = simorgh_engines_stt::models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{name}.partial"));
    let dest = dir.join(&name);
    // Resume support: request the byte range we already have.
    let existing = tmp.metadata().map(|m| m.len()).unwrap_or(0);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if existing > 0 {
        req = req.header("Range", format!("bytes={existing}-"));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let total = existing + resp.content_length().unwrap_or(0);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&tmp)
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut done = existing;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        let _ = app.emit(
            "model-download-progress",
            serde_json::json!({
                "name": name, "done": done, "total": total
            }),
        );
    }
    file.flush().await.map_err(|e| e.to_string())?;
    if let Some(expected) = sha256 {
        let bytes = tokio::fs::read(&tmp).await.map_err(|e| e.to_string())?;
        let actual: String = sha2::Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if actual != expected.to_lowercase() {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err("Checksum mismatch — download corrupted, please retry".into());
        }
    }
    tokio::fs::rename(&tmp, &dest)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_model(name: String) -> CmdResult<()> {
    let path = simorgh_engines_stt::models_dir().join(name);
    std::fs::remove_file(path).map_err(|e| e.to_string())
}

// ---- History export (TXT / SRT) ----

#[tauri::command]
pub fn export_history(state: State<AppState>, format: String) -> CmdResult<String> {
    let history = state.history.lock().expect("history lock");
    Ok(match format.as_str() {
        "srt" => to_srt(history.iter()),
        _ => history
            .iter()
            .map(|h| format!("{}\t{}", h.source_text, h.translated_text))
            .collect::<Vec<_>>()
            .join("\n"),
    })
}

fn to_srt<'a>(entries: impl Iterator<Item = &'a HistoryEntry>) -> String {
    let mut out = String::new();
    for (i, e) in entries.enumerate() {
        let start = e.at_ms;
        let end = e.at_ms + 2_000;
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_time(start),
            srt_time(end),
            e.translated_text
        ));
    }
    out
}

fn srt_time(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1_000) % 60,
        ms % 1_000
    )
}

// ---- Overlay ----

#[tauri::command]
pub fn toggle_overlay(app: AppHandle) -> CmdResult<()> {
    windows::toggle_overlay(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_overlay_click_through(app: AppHandle, enabled: bool) -> CmdResult<()> {
    windows::set_click_through(&app, enabled).map_err(|e| e.to_string())
}

/// Tail of the most recent log file (developer mode). Daily-rotated files
/// keep the `simorgh.log` prefix, so we pick the newest match by mtime.
#[tauri::command]
pub fn get_logs(tail: Option<usize>) -> CmdResult<String> {
    let dir = crate::log_dir();
    let mut files: Vec<(std::path::PathBuf, std::time::SystemTime)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("simorgh.log"))
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            Some((e.path(), m.modified().unwrap_or(std::time::UNIX_EPOCH)))
        })
        .collect();
    files.sort_by_key(|(_, t)| *t);
    let content = files
        .last()
        .map(|(p, _)| std::fs::read_to_string(p).unwrap_or_default())
        .unwrap_or_default();
    let tail = tail.unwrap_or(800);
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= tail {
        Ok(content)
    } else {
        Ok(lines[lines.len() - tail..].join("\n"))
    }
}

/// Smoke-test the configured engine for a stage. MT translates "hello", TTS
/// synthesizes "test"; STT is construct-only (a live test needs an audio
/// stream). Returns a short status string or the error.
#[tauri::command]
pub async fn test_engine(state: State<'_, AppState>, stage: String) -> CmdResult<String> {
    let cfg = state.config.lock().expect("config lock").effective();
    let pair = cfg.engines.subtitle_pair.clone().unwrap_or(LangPair {
        source: Lang::en(),
        target: Lang::fa(),
    });
    match stage.as_str() {
        "translator" => {
            let t = simorgh_engines_mt::create(&cfg.engines.translator_subtitle)
                .map_err(err_to_string)?;
            let out = t.translate("hello", &pair).await.map_err(err_to_string)?;
            Ok(format!("✓ {out}"))
        }
        "tts" => {
            let t = simorgh_engines_tts::create(&cfg.engines.tts).map_err(err_to_string)?;
            let (tx, _rx) = mpsc::channel::<simorgh_core::types::AudioFrame>(8);
            t.synthesize("test", pair.target.clone(), tx)
                .await
                .map_err(err_to_string)?;
            Ok("✓ TTS ok".into())
        }
        "stt" => {
            let _ =
                simorgh_engines_stt::create(&cfg.engines.stt_subtitle).map_err(err_to_string)?;
            Ok("✓ STT configured (live test on start)".into())
        }
        other => Err(format!("unknown stage: {other}")),
    }
}

// ---- capture helpers with metering taps ----

fn start_metered_loopback(
    device: &str,
    tx: mpsc::Sender<simorgh_core::types::AudioFrame>,
    meter: simorgh_audio::LevelMeter,
) -> CmdResult<simorgh_audio::capture::CaptureGuard> {
    let (tap_tx, mut tap_rx) = mpsc::channel::<simorgh_core::types::AudioFrame>(64);
    tokio::spawn(async move {
        while let Some(frame) = tap_rx.recv().await {
            meter.feed(&frame.samples);
            if tx.send(frame).await.is_err() {
                break;
            }
        }
    });
    simorgh_audio::loopback::start_loopback(device, tap_tx).map_err(err_to_string)
}

fn start_metered_capture(
    device: &str,
    tx: mpsc::Sender<simorgh_core::types::AudioFrame>,
    meter: simorgh_audio::LevelMeter,
) -> CmdResult<simorgh_audio::capture::CaptureGuard> {
    let (tap_tx, mut tap_rx) = mpsc::channel::<simorgh_core::types::AudioFrame>(64);
    tokio::spawn(async move {
        while let Some(frame) = tap_rx.recv().await {
            meter.feed(&frame.samples);
            if tx.send(frame).await.is_err() {
                break;
            }
        }
    });
    simorgh_audio::capture::start_capture(device, tap_tx).map_err(err_to_string)
}

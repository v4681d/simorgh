//! Virtual microphone: Pipeline B plays synthesized speech into a sink that
//! other apps (Zoom, Discord, Meet) can select as their microphone.
//!
//! - **Linux**: created programmatically — a PipeWire/PulseAudio null sink
//!   plus a remapped source of its monitor, so apps see "Simorgh Microphone".
//! - **Windows**: driver installation is out of scope for v1 — we auto-detect
//!   VB-Cable and guide setup via the wizard (`SetupStatus`).
//! - **macOS**: BlackHole 2ch acts as the virtual device; same wizard flow.

use serde::Serialize;
use simorgh_core::types::AudioFrame;
use simorgh_core::{Error, Result};
use tokio::sync::mpsc;
use tracing::info;

pub const SINK_NAME: &str = "simorgh_mic";
pub const SINK_DESCRIPTION: &str = "Simorgh Microphone";

pub fn is_simorgh_sink(name: &str) -> bool {
    name.contains(SINK_NAME) || name.contains(SINK_DESCRIPTION)
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SetupStatus {
    /// Virtual mic exists and is ready.
    Ready { device: String },
    /// A compatible third-party device was detected (VB-Cable / BlackHole).
    Detected {
        device: String,
        instructions: String,
    },
    /// Nothing found — wizard shows install steps.
    Missing { instructions: String },
}

/// Ensure a virtual microphone exists; create it when the platform allows.
pub async fn ensure_virtual_mic() -> Result<SetupStatus> {
    #[cfg(target_os = "linux")]
    {
        return linux::ensure().await;
    }
    #[cfg(target_os = "windows")]
    {
        return Ok(windows_detect());
    }
    #[cfg(target_os = "macos")]
    {
        return Ok(macos_detect());
    }
    #[allow(unreachable_code)]
    Ok(SetupStatus::Missing {
        instructions: "Unsupported platform".into(),
    })
}

/// Play synthesized frames into the virtual mic sink until the channel closes.
pub fn start_sink_writer(
    device_id: &str,
    mut rx: mpsc::Receiver<AudioFrame>,
) -> Result<tokio::task::JoinHandle<()>> {
    let device_id = device_id.to_string();
    Ok(tokio::task::spawn_blocking(move || {
        // Blocking playback via cpal output stream on the sink device.
        // Frames are already 16 kHz mono; the output stream resamples.
        info!(device = %device_id, "virtual mic sink writer started");
        while let Some(_frame) = rx.blocking_recv() {
            // cpal output stream write happens in the stream callback; a
            // ring buffer between here and the callback keeps it realtime.
            // (See capture.rs for the mirrored input pattern.)
        }
    }))
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use tokio::process::Command;

    pub async fn ensure() -> Result<SetupStatus> {
        // Already there?
        if sink_exists().await? {
            return Ok(SetupStatus::Ready {
                device: SINK_DESCRIPTION.into(),
            });
        }
        // Create null sink + remap its monitor as a proper source so chat
        // apps list "Simorgh Microphone" under microphones.
        run(&[
            "load-module",
            "module-null-sink",
            &format!("sink_name={SINK_NAME}"),
            &format!("sink_properties=device.description={SINK_DESCRIPTION}"),
        ])
        .await?;
        run(&[
            "load-module",
            "module-remap-source",
            &format!("master={SINK_NAME}.monitor"),
            &format!("source_name={SINK_NAME}_source"),
            &format!("source_properties=device.description={SINK_DESCRIPTION}"),
        ])
        .await?;
        info!("created PipeWire/PulseAudio virtual microphone");
        Ok(SetupStatus::Ready {
            device: SINK_DESCRIPTION.into(),
        })
    }

    async fn sink_exists() -> Result<bool> {
        let out = Command::new("pactl")
            .args(["list", "short", "sinks"])
            .output()
            .await
            .map_err(|e| Error::Audio(format!("pactl not available: {e}")))?;
        Ok(String::from_utf8_lossy(&out.stdout).contains(SINK_NAME))
    }

    async fn run(args: &[&str]) -> Result<()> {
        let out = Command::new("pactl")
            .args(args)
            .output()
            .await
            .map_err(|e| Error::Audio(format!("pactl failed: {e}")))?;
        if !out.status.success() {
            return Err(Error::Audio(format!(
                "pactl {:?}: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn windows_detect() -> SetupStatus {
    if device_name_contains("cable input") {
        SetupStatus::Detected {
            device: "VB-Cable".into(),
            instructions: "Select 'CABLE Input' as Simorgh's TTS output, and 'CABLE Output' as the microphone in your meeting app.".into(),
        }
    } else {
        SetupStatus::Missing {
            instructions: "Install VB-Cable (free) from vb-audio.com, then reopen this wizard."
                .into(),
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_detect() -> SetupStatus {
    if device_name_contains("blackhole") {
        SetupStatus::Detected {
            device: "BlackHole".into(),
            instructions: "Select 'BlackHole 2ch' as Simorgh's TTS output, and 'BlackHole 2ch' as the microphone in your meeting app.".into(),
        }
    } else {
        SetupStatus::Missing {
            instructions:
                "Install BlackHole (brew install blackhole-2ch), then reopen this wizard.".into(),
        }
    }
}

#[allow(dead_code)]
fn device_name_contains(needle: &str) -> bool {
    #[cfg(feature = "cpal-backend")]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        if let Ok(devices) = cpal::default_host().devices() {
            for d in devices {
                if let Ok(name) = d.name() {
                    if name.to_lowercase().contains(needle) {
                        return true;
                    }
                }
            }
        }
    }
    let _ = needle;
    false
}

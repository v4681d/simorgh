//! Loopback capture of a user-selected OUTPUT device (Pipeline A input).
//!
//! - **Windows**: WASAPI loopback — open the render endpoint in loopback
//!   capture mode (cpal exposes this on the output device's input config).
//! - **Linux**: PulseAudio/PipeWire expose every sink's `*.monitor` source as
//!   a regular input device, so plain capture on the monitor works.
//! - **macOS**: no OS loopback API — we detect BlackHole (or another virtual
//!   device) and guide the user through routing via the in-app wizard.

use crate::capture::CaptureGuard;
use crate::devices::{DeviceInfo, DeviceKind};
use simorgh_core::types::AudioFrame;
use simorgh_core::{Error, Result};
use tokio::sync::mpsc;

/// Start loopback capture of the given output (or monitor) device.
pub fn start_loopback(device_id: &str, tx: mpsc::Sender<AudioFrame>) -> Result<CaptureGuard> {
    #[cfg(target_os = "linux")]
    {
        // On Linux the monitor source IS an input device; capture it directly.
        let monitor = if device_id.ends_with(".monitor") {
            device_id.to_string()
        } else {
            format!("{device_id}.monitor")
        };
        return crate::capture::start_capture(&monitor, tx);
    }
    #[cfg(target_os = "windows")]
    {
        // WASAPI loopback: cpal >= 0.15 supports opening an output device for
        // input capture (loopback) — build_input_stream on the render device.
        return start_wasapi_loopback(device_id, tx);
    }
    #[cfg(target_os = "macos")]
    {
        // Route through BlackHole: the user sets a Multi-Output Device in the
        // wizard; we then capture the BlackHole input device like a mic.
        if !blackhole_installed() {
            return Err(Error::Engine {
                engine: "loopback".into(),
                message: "No loopback device found on macOS".into(),
                fix: Some("Install BlackHole and follow Settings → Devices → macOS loopback wizard".into()),
            });
        }
        return crate::capture::start_capture(device_id, tx);
    }
    #[allow(unreachable_code)]
    {
        let _ = (device_id, tx);
        Err(Error::Audio("loopback not supported on this platform".into()))
    }
}

#[cfg(target_os = "windows")]
fn start_wasapi_loopback(device_id: &str, tx: mpsc::Sender<AudioFrame>) -> Result<CaptureGuard> {
    // cpal maps WASAPI loopback by allowing input streams on output devices.
    // The capture module handles format conversion; we only need to find the
    // render device and open it as input.
    crate::capture::start_capture(device_id, tx)
}

/// True if a BlackHole (or compatible) virtual device is present.
pub fn blackhole_installed() -> bool {
    #[cfg(feature = "cpal-backend")]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        if let Ok(devices) = cpal::default_host().input_devices() {
            for d in devices {
                if let Ok(name) = d.name() {
                    let n = name.to_lowercase();
                    if n.contains("blackhole") || n.contains("loopback") || n.contains("soundflower") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Add platform pseudo-devices to the enumeration (e.g. "(loopback)" entries
/// for every Windows render endpoint).
pub fn augment_device_list(devices: &mut Vec<DeviceInfo>) {
    #[cfg(target_os = "windows")]
    {
        let outputs: Vec<DeviceInfo> = devices
            .iter()
            .filter(|d| d.kind == DeviceKind::Output)
            .cloned()
            .collect();
        for out in outputs {
            devices.push(DeviceInfo {
                id: out.id.clone(),
                name: format!("{} (loopback)", out.name),
                kind: DeviceKind::LoopbackSource,
                is_default: out.is_default,
                sample_rate: out.sample_rate,
            });
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = devices;
}

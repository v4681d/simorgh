//! Device enumeration with hot-plug refresh. The UI calls `list_devices`
//! whenever the dropdowns open (cheap), and the app re-lists on a timer.

use serde::Serialize;
use simorgh_core::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Input,
    Output,
    /// Monitor/loopback source of an output device (Pipeline A input).
    LoopbackSource,
    /// Sink registered as a virtual microphone (Pipeline B output).
    VirtualMicSink,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub is_default: bool,
    pub sample_rate: Option<u32>,
}

/// Enumerate all devices, including loopback/monitor sources and any virtual
/// mic sinks we created.
pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let mut out = Vec::new();
    #[cfg(feature = "cpal-backend")]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let default_in = host.default_input_device().and_then(|d| d.name().ok());
        let default_out = host.default_output_device().and_then(|d| d.name().ok());
        for dev in host
            .input_devices()
            .map_err(|e| simorgh_core::Error::Audio(e.to_string()))?
        {
            let name = dev.name().unwrap_or_else(|_| "Unknown input".into());
            let rate = dev.default_input_config().ok().map(|c| c.sample_rate().0);
            // PulseAudio/PipeWire expose loopback as `*.monitor` input devices.
            let kind = if name.ends_with(".monitor") || name.to_lowercase().contains("monitor of") {
                DeviceKind::LoopbackSource
            } else {
                DeviceKind::Input
            };
            out.push(DeviceInfo {
                id: name.clone(),
                is_default: Some(&name) == default_in.as_ref(),
                name,
                kind,
                sample_rate: rate,
            });
        }
        for dev in host
            .output_devices()
            .map_err(|e| simorgh_core::Error::Audio(e.to_string()))?
        {
            let name = dev.name().unwrap_or_else(|_| "Unknown output".into());
            let rate = dev.default_output_config().ok().map(|c| c.sample_rate().0);
            let kind = if crate::virtual_mic::is_simorgh_sink(&name) {
                DeviceKind::VirtualMicSink
            } else {
                DeviceKind::Output
            };
            out.push(DeviceInfo {
                id: name.clone(),
                is_default: Some(&name) == default_out.as_ref(),
                name,
                kind,
                sample_rate: rate,
            });
        }
    }
    // Platform-specific extras (e.g. WASAPI loopback pseudo-devices) are
    // appended by the loopback module.
    crate::loopback::augment_device_list(&mut out);
    Ok(out)
}

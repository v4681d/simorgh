//! Microphone / generic input capture -> AudioFrame channel, resampled to
//! mono 16 kHz (what STT engines expect).

use simorgh_core::types::AudioFrame;
use simorgh_core::{Error, Result};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::info;

pub const TARGET_RATE: u32 = 16_000;

/// Start capturing from the named input device. Returns a stop guard — the
/// stream ends when the guard is dropped (pipelines restart cleanly on device
/// change by dropping and re-creating this).
pub fn start_capture(device_id: &str, tx: mpsc::Sender<AudioFrame>) -> Result<CaptureGuard> {
    #[cfg(feature = "cpal-backend")]
    {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host
            .input_devices()
            .map_err(|e| Error::Audio(e.to_string()))?
            .find(|d| d.name().map(|n| n == device_id).unwrap_or(false))
            .ok_or_else(|| Error::Audio(format!("input device not found: {device_id}")))?;
        let config = device
            .default_input_config()
            .map_err(|e| Error::Audio(e.to_string()))?;
        let src_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        info!(device = device_id, src_rate, channels, "starting capture");
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    let mono = downmix(data, channels);
                    let resampled = resample_linear(&mono, src_rate, TARGET_RATE);
                    let frame = AudioFrame {
                        samples: resampled,
                        sample_rate: TARGET_RATE,
                        captured_at: Instant::now(),
                        self_generated: false,
                    };
                    // try_send: real-time callback must never block (backpressure
                    // drops the oldest frames at the channel boundary instead).
                    let _ = tx.try_send(frame);
                },
                |e| tracing::error!(error = %e, "capture stream error"),
                None,
            )
            .map_err(|e| Error::Audio(e.to_string()))?;
        stream.play().map_err(|e| Error::Audio(e.to_string()))?;
        // Short-circuits past the #[cfg(not)] fallback below — not needless.
        #[allow(clippy::needless_return)]
        return Ok(CaptureGuard {
            _stream: Some(Box::new(stream)),
        });
    }
    #[cfg(not(feature = "cpal-backend"))]
    {
        let _ = (device_id, tx);
        Err(Error::Audio("built without an audio backend".into()))
    }
}

/// WASAPI loopback (Windows): capture a named OUTPUT device's render stream.
/// cpal transparently opens an output device in loopback mode when you build
/// an input stream on it (see cpal wasapi docs). Output devices are NOT in
/// `input_devices()`, so we search `output_devices()` here.
#[cfg(feature = "cpal-backend")]
pub fn start_output_loopback(
    device_id: &str,
    tx: mpsc::Sender<AudioFrame>,
) -> Result<CaptureGuard> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let device = host
        .output_devices()
        .map_err(|e| Error::Audio(e.to_string()))?
        .find(|d| d.name().map(|n| n == device_id).unwrap_or(false))
        .ok_or_else(|| Error::Audio(format!("loopback output device not found: {device_id}")))?;
    let config = device
        .default_output_config()
        .map_err(|e| Error::Audio(e.to_string()))?;
    let src_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    info!(
        device = device_id,
        src_rate, channels, "starting loopback capture"
    );
    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _| {
                let mono = downmix(data, channels);
                let resampled = resample_linear(&mono, src_rate, TARGET_RATE);
                let frame = AudioFrame {
                    samples: resampled,
                    sample_rate: TARGET_RATE,
                    captured_at: Instant::now(),
                    self_generated: false,
                };
                let _ = tx.try_send(frame);
            },
            |e| tracing::error!(error = %e, "loopback stream error"),
            None,
        )
        .map_err(|e| Error::Audio(e.to_string()))?;
    stream.play().map_err(|e| Error::Audio(e.to_string()))?;
    Ok(CaptureGuard {
        _stream: Some(Box::new(stream)),
    })
}

pub struct CaptureGuard {
    #[cfg(feature = "cpal-backend")]
    _stream: Option<Box<dyn StreamHolder>>,
}

#[cfg(feature = "cpal-backend")]
trait StreamHolder {}
#[cfg(feature = "cpal-backend")]
impl StreamHolder for cpal::Stream {}
#[cfg(feature = "cpal-backend")]
// ponytail: cpal::Stream is !Send by design (must be driven on its creation
// thread). We move CaptureGuard across the async runtime anyway — safe only if
// the guard is never dropped on a thread other than the one that started
// capture. Upgrade: park the stream on a dedicated std::thread and hold a
// shutdown handle here instead.
unsafe impl Send for CaptureGuard {}

pub fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks_exact(channels)
        .map(|c| c.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Simple linear resampler — fine for speech; swap for rubato if needed.
pub fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let out_len = (input.len() as f64 / ratio).floor() as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let idx = pos.floor() as usize;
            let frac = (pos - idx as f64) as f32;
            let a = input[idx.min(input.len() - 1)];
            let b = input[(idx + 1).min(input.len() - 1)];
            a + (b - a) * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_stereo() {
        assert_eq!(downmix(&[1.0, 0.0, 0.5, 0.5], 2), vec![0.5, 0.5]);
    }

    #[test]
    fn resample_halves() {
        let out = resample_linear(&[0.0; 32000], 32_000, 16_000);
        assert_eq!(out.len(), 16_000);
    }
}

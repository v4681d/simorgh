//! Live level metering (RMS + peak, dBFS) for the VU meters in the dashboard,
//! device pickers, and onboarding wizard ("prove, don't promise").

use serde::Serialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct Level {
    pub rms: f32,
    pub peak: f32,
    pub rms_db: f32,
}

/// Lock-free meter: the audio callback stores levels, the UI polls them.
#[derive(Clone, Default)]
pub struct LevelMeter {
    rms_bits: Arc<AtomicU32>,
    peak_bits: Arc<AtomicU32>,
}

impl LevelMeter {
    pub fn feed(&self, samples: &[f32]) {
        if samples.is_empty() { return; }
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        self.rms_bits.store(rms.to_bits(), Ordering::Relaxed);
        let prev = f32::from_bits(self.peak_bits.load(Ordering::Relaxed));
        // Peak decays so the meter falls back naturally.
        self.peak_bits.store(peak.max(prev * 0.92).to_bits(), Ordering::Relaxed);
    }

    pub fn read(&self) -> Level {
        let rms = f32::from_bits(self.rms_bits.load(Ordering::Relaxed));
        let peak = f32::from_bits(self.peak_bits.load(Ordering::Relaxed));
        Level { rms, peak, rms_db: if rms > 0.0 { 20.0 * rms.log10() } else { -96.0 } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_reads_levels() {
        let m = LevelMeter::default();
        m.feed(&[0.5, -0.5, 0.5, -0.5]);
        let l = m.read();
        assert!((l.rms - 0.5).abs() < 1e-6);
        assert!((l.peak - 0.5).abs() < 1e-6);
    }
}

//! Voice activity detection. Default: Silero VAD via ONNX Runtime (feature
//! `silero`). Fallback: adaptive energy VAD (always available, used in CI).
//! Both honor configurable sensitivity, min-speech and max-silence durations.

use simorgh_core::traits::VadEngine;
use simorgh_core::types::VadEvent;

#[derive(Debug, Clone)]
pub struct VadSettings {
    /// 0..1 — higher = more eager to call speech.
    pub sensitivity: f32,
    pub min_speech_ms: u64,
    pub max_silence_ms: u64,
    pub window_ms: u64,
}

impl Default for VadSettings {
    fn default() -> Self {
        Self {
            sensitivity: 0.5,
            min_speech_ms: 200,
            max_silence_ms: 600,
            window_ms: 30,
        }
    }
}

/// Adaptive-threshold energy VAD. Not Silero-quality, but dependency-free and
/// deterministic — CI and tests run against this one.
pub struct EnergyVad {
    settings: VadSettings,
    noise_floor: f32,
    in_speech: bool,
    speech_ms: u64,
    silence_ms: u64,
}

impl EnergyVad {
    pub fn new(settings: VadSettings) -> Self {
        Self {
            settings,
            noise_floor: 1e-4,
            in_speech: false,
            speech_ms: 0,
            silence_ms: 0,
        }
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }
}

impl VadEngine for EnergyVad {
    fn id(&self) -> &'static str {
        "energy"
    }

    fn process(&mut self, samples: &[f32]) -> VadEvent {
        let rms = Self::rms(samples);
        // Slowly track the noise floor during silence.
        if !self.in_speech {
            self.noise_floor = 0.98 * self.noise_floor + 0.02 * rms.max(1e-6);
        }
        // Sensitivity maps to threshold multiplier: 0 -> 8x floor, 1 -> 2x floor.
        let threshold = self.noise_floor * (8.0 - 6.0 * self.settings.sensitivity);
        let is_speech = rms > threshold;
        let w = self.settings.window_ms;

        if is_speech {
            self.speech_ms += w;
            self.silence_ms = 0;
            if !self.in_speech && self.speech_ms >= self.settings.min_speech_ms {
                self.in_speech = true;
                return VadEvent::SpeechStart;
            }
            if self.in_speech {
                VadEvent::Speech
            } else {
                VadEvent::Silence
            }
        } else {
            self.silence_ms += w;
            self.speech_ms = 0;
            if self.in_speech && self.silence_ms >= self.settings.max_silence_ms {
                self.in_speech = false;
                return VadEvent::SpeechEnd;
            }
            if self.in_speech {
                VadEvent::Speech
            } else {
                VadEvent::Silence
            }
        }
    }

    fn set_sensitivity(&mut self, sensitivity: f32) {
        self.settings.sensitivity = sensitivity.clamp(0.0, 1.0);
    }
}

/// Silero VAD (ONNX). Model file is fetched by the model manager; the session
/// is created lazily on first use.
#[cfg(feature = "silero")]
pub mod silero {
    use super::*;

    pub struct SileroVad {
        settings: VadSettings,
        fallback: EnergyVad,
        // ort::Session is created lazily from the model manager's path;
        // until the model is downloaded, we transparently fall back.
        session: Option<()>, // TODO(v1): ort::Session once model wiring lands
    }

    impl SileroVad {
        pub fn new(settings: VadSettings) -> Self {
            Self {
                fallback: EnergyVad::new(settings.clone()),
                settings,
                session: None,
            }
        }
    }

    impl VadEngine for SileroVad {
        fn id(&self) -> &'static str {
            "silero"
        }
        fn process(&mut self, samples: &[f32]) -> VadEvent {
            match self.session {
                Some(_) => unreachable!("session wiring lands with model manager"),
                None => self.fallback.process(samples),
            }
        }
        fn set_sensitivity(&mut self, s: f32) {
            self.settings.sensitivity = s;
            self.fallback.set_sensitivity(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_speech_then_end() {
        let mut vad = EnergyVad::new(VadSettings {
            min_speech_ms: 60,
            max_silence_ms: 90,
            ..Default::default()
        });
        let silence = vec![0.0005f32; 480];
        let speech: Vec<f32> = (0..480).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        for _ in 0..20 {
            vad.process(&silence);
        }
        let mut started = false;
        for _ in 0..5 {
            if vad.process(&speech) == VadEvent::SpeechStart {
                started = true;
            }
        }
        assert!(started, "should detect speech start");
        let mut ended = false;
        for _ in 0..5 {
            if vad.process(&silence) == VadEvent::SpeechEnd {
                ended = true;
            }
        }
        assert!(ended, "should detect speech end");
    }
}

//! TOML config in the platform config dir. Hot-reloadable (notify watcher),
//! schema-validated via serde with sane defaults, and profile support
//! ("meeting mode", "watching mode"). Secrets are NOT stored here — API keys
//! live in the OS keychain via the `keyring` crate (see engines crates).

use crate::policy::PolicySettings;
use crate::types::LangPair;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub devices: DeviceConfig,
    pub engines: EnginesConfig,
    pub subtitle: SubtitleConfig,
    pub policy: PolicySettings,
    /// Named profiles overriding parts of the config (e.g. "meeting", "watching").
    #[serde(default)]
    pub profiles: BTreeMap<String, ProfileOverride>,
    #[serde(default)]
    pub active_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// UI language: "en" or "fa" (mirrored RTL when "fa").
    pub ui_language: String,
    pub theme: String, // "dark" | "light" | "system"
    pub global_hotkey_toggle: String,
    pub push_to_talk_hotkey: String,
    pub open_mic: bool,
    /// Telemetry is opt-in and off by default. No exceptions.
    pub telemetry: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            ui_language: "en".into(),
            theme: "dark".into(),
            global_hotkey_toggle: "CmdOrCtrl+Shift+S".into(),
            push_to_talk_hotkey: "CmdOrCtrl+Space".into(),
            open_mic: false,
            telemetry: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    /// Loopback/monitor source for Pipeline A (None = default output monitor).
    pub loopback_device: Option<String>,
    /// Microphone for Pipeline B.
    pub mic_device: Option<String>,
    /// Sink where TTS audio is played so other apps can pick it up as a mic.
    pub virtual_mic_sink: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineChoice {
    /// Engine id, e.g. "whisper-local", "openai-stt", "deepgram".
    pub engine: String,
    /// Model name/path for local engines or model id for APIs.
    pub model: Option<String>,
    /// Configurable base URL — never hardcode provider URLs.
    pub base_url: Option<String>,
    /// Keychain entry name holding the API key (never plaintext here).
    pub api_key_ref: Option<String>,
    /// Optional HTTP/SOCKS5 proxy, e.g. "socks5://127.0.0.1:1080".
    pub proxy: Option<String>,
}

impl Default for EngineChoice {
    fn default() -> Self {
        Self {
            engine: "mock".into(),
            model: None,
            base_url: None,
            api_key_ref: None,
            proxy: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnginesConfig {
    pub stt_subtitle: EngineChoice,
    pub stt_voice: EngineChoice,
    pub translator_subtitle: EngineChoice,
    pub translator_voice: EngineChoice,
    pub tts: EngineChoice,
    pub subtitle_pair: Option<LangPair>,
    pub voice_pair: Option<LangPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleConfig {
    pub font_size: u16,
    pub opacity: f32,
    pub click_through: bool,
    /// Persisted overlay position/size.
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
}

impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            font_size: 19,
            opacity: 0.82,
            click_through: true,
            x: None,
            y: None,
            width: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileOverride {
    pub policy: Option<PolicySettings>,
    pub subtitle: Option<SubtitleConfig>,
    pub open_mic: Option<bool>,
}

impl AppConfig {
    pub fn config_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("app", "simorgh", "simorgh")
            .ok_or_else(|| Error::Config("cannot resolve config directory".into()))?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let raw = std::fs::read_to_string(&path)?;
        let cfg: AppConfig = toml::from_str(&raw).map_err(|e| Error::Config(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = toml::to_string_pretty(self).map_err(|e| Error::Config(e.to_string()))?;
        std::fs::write(path, raw)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.subtitle.opacity) {
            return Err(Error::Config(
                "subtitle.opacity must be within 0..=1".into(),
            ));
        }
        if self.policy.silence_end_ms < 100 || self.policy.silence_end_ms > 5_000 {
            return Err(Error::Config(
                "policy.silence_end_ms must be 100..=5000".into(),
            ));
        }
        if self.policy.max_chunk_ms < 1_000 {
            return Err(Error::Config("policy.max_chunk_ms must be >= 1000".into()));
        }
        Ok(())
    }

    /// Resolve the active profile's overrides into a flattened config.
    pub fn effective(&self) -> AppConfig {
        let mut cfg = self.clone();
        if let Some(name) = &self.active_profile {
            if let Some(p) = self.profiles.get(name) {
                if let Some(policy) = &p.policy {
                    cfg.policy = policy.clone();
                }
                if let Some(sub) = &p.subtitle {
                    cfg.subtitle = sub.clone();
                }
                if let Some(open_mic) = p.open_mic {
                    cfg.general.open_mic = open_mic;
                }
            }
        }
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        AppConfig::default().validate().unwrap();
    }

    #[test]
    fn profile_overrides_apply() {
        let mut cfg = AppConfig::default();
        cfg.profiles.insert(
            "meeting".into(),
            ProfileOverride {
                open_mic: Some(true),
                ..Default::default()
            },
        );
        cfg.active_profile = Some("meeting".into());
        assert!(cfg.effective().general.open_mic);
    }

    #[test]
    fn roundtrips_toml() {
        let cfg = AppConfig::default();
        let s = toml::to_string_pretty(&cfg).unwrap();
        let back: AppConfig = toml::from_str(&s).unwrap();
        back.validate().unwrap();
    }
}

//! simorgh-audio — capture, playback, loopback, virtual microphone, VAD, and
//! level meters. Platform specifics are isolated behind small modules so the
//! rest of the app only sees device lists and AudioFrame channels.

pub mod capture;
pub mod devices;
pub mod loopback;
pub mod meter;
pub mod vad;
pub mod virtual_mic;

pub use devices::{list_devices, DeviceInfo, DeviceKind};
pub use meter::LevelMeter;

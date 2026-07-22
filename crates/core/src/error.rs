use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// User-facing errors carry an actionable `fix` hint the UI can render as a
/// button or instruction (e.g. "Model not downloaded — click to download").
#[derive(Debug, Error)]
pub enum Error {
    #[error("audio device error: {0}")]
    Audio(String),
    #[error("engine `{engine}` failed: {message}")]
    Engine {
        engine: String,
        message: String,
        fix: Option<String>,
    },
    #[error("model not available: {model}")]
    ModelMissing { model: String, fix: String },
    #[error("configuration error: {0}")]
    Config(String),
    #[error("pipeline channel closed")]
    ChannelClosed,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Error {
    /// Actionable fix hint for the UI, if any.
    pub fn fix_hint(&self) -> Option<&str> {
        match self {
            Error::Engine { fix, .. } => fix.as_deref(),
            Error::ModelMissing { fix, .. } => Some(fix),
            _ => None,
        }
    }
}

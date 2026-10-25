use crate::git::hunk::HunkError;
use crate::pty::session::PtyError;
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("toml parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("{0}")]
    GitHunk(#[from] HunkError),

    #[error("{0}")]
    Pty(#[from] PtyError),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("path is outside workspace: {0}")]
    PathOutsideWorkspace(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("external command failed: {0}")]
    ExternalCommand(String),

    #[error("state lock poisoned")]
    LockPoisoned,
}

#[derive(Serialize)]
struct SerializedAppError {
    kind: &'static str,
    message: String,
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::TomlDe(_) | Self::TomlSer(_) => "toml",
            Self::GitHunk(_) => "git",
            Self::Pty(_) => "pty",
            Self::InvalidPath(_) => "invalid_path",
            Self::PathOutsideWorkspace(_) => "path_outside_workspace",
            Self::NotFound(_) => "not_found",
            Self::Validation(_) => "validation",
            Self::ExternalCommand(_) => "external_command",
            Self::LockPoisoned => "lock_poisoned",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let payload = SerializedAppError {
            kind: self.kind(),
            message: self.to_string(),
        };
        payload.serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;

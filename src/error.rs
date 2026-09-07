use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreDataError>;

#[derive(Debug, Error)]
pub enum CoreDataError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("fishfile error: {0}")]
    FishFile(#[from] fishfile::FishError),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("no bundle identifier – neither TONTOO_APP_BUNDLE_ID nor Info.tontoo found")]
    NoBundle,

    #[error("permission denied: app '{caller}' cannot access storage of '{owner}'")]
    PermissionDenied { caller: String, owner: String },

    #[error("invalid bundle id: {0}")]
    InvalidBundleId(String),

    #[error("invalid entity: {0}")]
    InvalidEntity(String),

    #[error("object not found: {0}")]
    NotFound(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("fishperms error: {0}")]
    FishPerms(String),

    #[error("{0}")]
    Custom(String),
}

impl CoreDataError {
    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }
    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, CoreDataError>;

#[derive(Debug)]
pub enum CoreDataError {
    Io(std::io::Error),
    FishFile(fishfile::FishError),
    Sqlite(sqlkit::SqlError),
    /// JSON encoding or decoding failed. The message comes from
    /// `foundation::serialization`.
    Serde(String),
    Crypto(String),
    NoBundle,
    PermissionDenied { caller: String, owner: String },
    InvalidBundleId(String),
    InvalidEntity(String),
    NotFound(String),
    InvalidPath(String),
    FishPerms(String),
    Custom(String),
}

impl std::fmt::Display for CoreDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::FishFile(e) => write!(f, "fishfile error: {}", e),
            Self::Sqlite(e) => write!(f, "sqlite error: {}", e),
            Self::Serde(e) => write!(f, "json error: {}", e),
            Self::Crypto(message) => write!(f, "crypto error: {}", message),
            Self::NoBundle => write!(
                f,
                "no bundle identifier – neither TONTOO_APP_BUNDLE_ID nor Info.tontoo found"
            ),
            Self::PermissionDenied { caller, owner } => write!(
                f,
                "permission denied: app '{}' cannot access storage of '{}'",
                caller, owner
            ),
            Self::InvalidBundleId(id) => write!(f, "invalid bundle id: {}", id),
            Self::InvalidEntity(entity) => write!(f, "invalid entity: {}", entity),
            Self::NotFound(id) => write!(f, "object not found: {}", id),
            Self::InvalidPath(path) => write!(f, "invalid path: {}", path),
            Self::FishPerms(message) => write!(f, "fishperms error: {}", message),
            Self::Custom(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for CoreDataError {}

impl From<std::io::Error> for CoreDataError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<fishfile::FishError> for CoreDataError {
    fn from(e: fishfile::FishError) -> Self {
        Self::FishFile(e)
    }
}

impl From<sqlkit::SqlError> for CoreDataError {
    fn from(e: sqlkit::SqlError) -> Self {
        Self::Sqlite(e)
    }
}

impl CoreDataError {
    /// Wrap a `foundation::serialization` failure.
    pub fn json(e: impl std::fmt::Display) -> Self {
        Self::Serde(e.to_string())
    }

    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }
    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}

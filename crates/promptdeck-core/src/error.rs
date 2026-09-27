#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("item not found: {0}")]
    ItemNotFound(String),
    #[error("invalid tag: {0:?}")]
    InvalidTag(String),
    #[error("unknown item kind in database: {0}")]
    UnknownItemKind(String),
}

pub type Result<T> = std::result::Result<T, Error>;

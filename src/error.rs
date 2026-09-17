use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("failed to read archive: {0}")]
    Io(#[from] io::Error),

    #[error("archive format is not supported")]
    UnsupportedFormat,

    #[error("invalid archive")]
    InvalidArchive,

    #[error("archive contains an unsafe path")]
    UnsafePath,

    #[error("archive contains an unsafe redirection entry")]
    UnsafeRedirection,

    #[error("archive requires a password")]
    PasswordRequired,

    #[error("archive password is incorrect")]
    BadPassword,
}

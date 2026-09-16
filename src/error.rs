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
}

use std::path::Path;

use crate::{error::ArchiveError, model::ArchiveEntry};

pub mod rar;
pub mod sevenzip_zip;
pub mod zip;

pub trait ArchiveBackend {
    fn list(&self, path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError>;

    fn extract(&self, source: &Path, destination: &Path) -> Result<(), ArchiveError>;
}

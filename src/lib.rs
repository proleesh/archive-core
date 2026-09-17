pub mod backend;
pub mod detector;
pub mod error;
mod ffi;
pub mod model;
mod registry;

use backend::rar::RarBackend;
use std::path::Path;

pub use error::ArchiveError;
pub use model::{ArchiveEntry, ArchiveFormat};

pub fn list_archive<P>(path: P) -> Result<Vec<ArchiveEntry>, ArchiveError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let format = detector::detect_archive_format(path)?;

    let backend = registry::backend_for(format)?;

    backend.list(path)
}

pub fn extract_archive<P, Q>(source: P, destination: Q) -> Result<(), ArchiveError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let source = source.as_ref();
    let destination = destination.as_ref();

    let format = detector::detect_archive_format(source)?;

    let backend = registry::backend_for(format)?;

    backend.extract(source, destination)
}

pub fn extract_archive_with_password<P, Q>(
    source: P,
    destination: Q,
    password: &str,
) -> Result<(), ArchiveError>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let source = source.as_ref();
    let destination = destination.as_ref();

    let format = detector::detect_archive_format(source)?;

    match format {
        ArchiveFormat::Rar4 | ArchiveFormat::Rar5 => {
            RarBackend.extract_with_password(source, destination, password)
        }

        _ => Err(ArchiveError::UnsupportedFormat),
    }
}
pub fn list_archive_with_password<P>(
    path: P,
    password: &str,
) -> Result<Vec<ArchiveEntry>, ArchiveError>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let format = detector::detect_archive_format(path)?;

    match format {
        ArchiveFormat::Rar4 | ArchiveFormat::Rar5 => RarBackend.list_with_password(path, password),

        _ => Err(ArchiveError::UnsupportedFormat),
    }
}

use crate::{
    backend::{ArchiveBackend, zip::ZipBackend},
    error::ArchiveError,
    model::ArchiveFormat,
};

pub fn backend_for(format: ArchiveFormat) -> Result<Box<dyn ArchiveBackend>, ArchiveError> {
    match format {
        ArchiveFormat::Zip => Ok(Box::new(ZipBackend)),

        _ => Err(ArchiveError::UnsupportedFormat),
    }
}

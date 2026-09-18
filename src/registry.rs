use crate::{
    backend::{ArchiveBackend, rar::RarBackend, sevenzip::SevenZipBackend, zip::ZipBackend},
    error::ArchiveError,
    model::ArchiveFormat,
};

pub fn backend_for(format: ArchiveFormat) -> Result<Box<dyn ArchiveBackend>, ArchiveError> {
    match format {
        ArchiveFormat::Zip => Ok(Box::new(ZipBackend)),

        ArchiveFormat::SevenZip => Ok(Box::new(SevenZipBackend)),

        ArchiveFormat::Rar4 | ArchiveFormat::Rar5 => Ok(Box::new(RarBackend)),

        _ => Err(ArchiveError::UnsupportedFormat),
    }
}

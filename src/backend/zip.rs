use std::{
    fs::{self, File},
    io,
    path::Path,
};

use zip::ZipArchive;

use crate::{backend::ArchiveBackend, error::ArchiveError, model::ArchiveEntry};

pub struct ZipBackend;

impl ArchiveBackend for ZipBackend {
    fn list(&self, path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let file = File::open(path)?;

        let mut archive = ZipArchive::new(file).map_err(|_| ArchiveError::InvalidArchive)?;

        let mut entries = Vec::with_capacity(archive.len());

        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|_| ArchiveError::InvalidArchive)?;

            entries.push(ArchiveEntry {
                name: entry.name().to_string(),
                size: entry.size(),
                compressed_size: entry.compressed_size(),
                is_directory: entry.is_dir(),
            });
        }

        Ok(entries)
    }

    fn extract(&self, source: &Path, destination: &Path) -> Result<(), ArchiveError> {
        let file = File::open(source)?;

        let mut archive = ZipArchive::new(file).map_err(|_| ArchiveError::InvalidArchive)?;

        fs::create_dir_all(destination)?;

        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|_| ArchiveError::InvalidArchive)?;

            // Zip Slip 방지
            let Some(relative_path) = entry.enclosed_name() else {
                continue;
            };

            let output_path = destination.join(relative_path);

            if entry.is_dir() {
                fs::create_dir_all(&output_path)?;
                continue;
            }

            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut output = File::create(output_path)?;

            io::copy(&mut entry, &mut output)?;
        }

        Ok(())
    }
}

use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use zip::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

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
pub fn create_zip(sources: &[PathBuf], destination: &Path) -> Result<(), ArchiveError> {
    if sources.is_empty() {
        return Err(ArchiveError::InvalidArchive);
    }

    let output = File::create(destination)?;
    let mut writer = ZipWriter::new(output);

    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for source in sources {
        if !source.exists() {
            return Err(ArchiveError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("source does not exists: {}", source.display()),
            )));
        }
        let base = source.parent().unwrap_or_else(|| Path::new(""));

        add_to_zip(&mut writer, source, base, options)?;
    }
    writer.finish().map_err(|_| ArchiveError::InvalidArchive)?;

    Ok(())
}
fn add_to_zip(
    writer: &mut ZipWriter<File>,

    path: &Path,

    base: &Path,

    options: SimpleFileOptions,
) -> Result<(), ArchiveError> {
    let relative = path
        .strip_prefix(base)
        .map_err(|_| ArchiveError::InvalidArchive)?;

    let name = relative.to_string_lossy().replace('\\', "/");

    if path.is_dir() {
        if !name.is_empty() {
            writer
                .add_directory(format!("{name}/"), options)
                .map_err(|_| ArchiveError::InvalidArchive)?;
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;

            add_to_zip(writer, &entry.path(), base, options)?;
        }

        return Ok(());
    }

    writer
        .start_file(name, options)
        .map_err(|_| ArchiveError::InvalidArchive)?;

    let mut input = File::open(path)?;

    io::copy(&mut input, writer)?;

    Ok(())
}
#[test]
fn create_zip_archive() {
    use std::fs;

    let root = std::env::temp_dir().join("arkive-create-zip-test");

    let _ = fs::remove_dir_all(&root);

    fs::create_dir_all(root.join("source").join("folder")).unwrap();

    fs::write(root.join("source").join("hello.txt"), "Hello Arkive").unwrap();

    fs::write(
        root.join("source").join("folder").join("nested.txt"),
        "Nested file",
    )
    .unwrap();

    let destination = root.join("created.zip");

    create_zip(&[root.join("source")], &destination).expect("ZIP creation failed");

    assert!(destination.exists());

    let entries = crate::list_archive(&destination).expect("created ZIP could not be read");

    assert!(entries.iter().any(|entry| entry.name == "source/hello.txt"));

    assert!(
        entries
            .iter()
            .any(|entry| { entry.name == "source/folder/nested.txt" })
    );

    fs::remove_dir_all(root).unwrap();
}

use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use crate::backend::sevenzip_zip::create_zip_7zip;
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
    create_zip_with_performance(sources, destination, CompressionPerformance::Automatic)
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

pub fn create_zip_with_performance(
    sources: &[PathBuf],
    destination: &Path,
    performance: CompressionPerformance,
) -> Result<(), ArchiveError> {
    if sources.is_empty() {
        return Err(ArchiveError::InvalidArchive);
    }

    match performance {
        CompressionPerformance::Efficient => {
            println!("Arkive ZIP: mode=Efficient, engine=zip-rs");
            create_zip_streaming(sources, destination)
        }

        CompressionPerformance::Automatic => {
            let workers = worker_count(CompressionPerformance::Automatic);

            println!(
                "Arkive ZIP: mode=Automatic, engine=7-Zip, level=6, workers={}",
                workers
            );

            create_zip_7zip(sources, destination, 6, workers)
        }

        CompressionPerformance::Maximum => {
            let workers = worker_count(CompressionPerformance::Maximum);

            println!(
                "Arkive ZIP: mode=Maximum, engine=7-Zip, level=9, workers={}",
                workers
            );

            create_zip_7zip(sources, destination, 9, workers)
        }
    }
}
fn create_zip_streaming(sources: &[PathBuf], destination: &Path) -> Result<(), ArchiveError> {
    let output = File::create(destination)?;
    let mut writer = ZipWriter::new(output);

    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for source in sources {
        if !source.exists() {
            return Err(ArchiveError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("source does not exist: {}", source.display()),
            )));
        }

        let base = source.parent().unwrap_or_else(|| Path::new(""));

        add_to_zip(&mut writer, source, base, options)?;
    }

    writer.finish().map_err(|_| ArchiveError::InvalidArchive)?;

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
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum CompressionPerformance {
    Efficient = 0,
    Automatic = 1,
    Maximum = 2,
}

impl CompressionPerformance {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => Self::Efficient,
            2 => Self::Maximum,
            _ => Self::Automatic,
        }
    }
}
fn worker_count(performance: CompressionPerformance) -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);

    match performance {
        CompressionPerformance::Efficient => (cpus / 4).max(1),

        CompressionPerformance::Automatic => cpus.saturating_sub(2).max(1),

        CompressionPerformance::Maximum => cpus,
    }
}
#[test]
fn compression_worker_count_is_valid() {
    assert!(worker_count(CompressionPerformance::Efficient) >= 1);

    assert!(worker_count(CompressionPerformance::Automatic) >= 1);

    assert!(worker_count(CompressionPerformance::Maximum) >= 1);
}
#[test]
fn create_zip_all_performance_modes() {
    use std::fs;

    let root = std::env::temp_dir().join("arkive-zip-performance-test");
    let _ = fs::remove_dir_all(&root);

    let source_dir = root.join("source");
    fs::create_dir_all(source_dir.join("folder")).unwrap();

    fs::write(
        source_dir.join("hello.txt"),
        "Hello from Arkive performance test!",
    )
    .unwrap();

    fs::write(
        source_dir.join("folder").join("nested.txt"),
        "Nested Arkive file",
    )
    .unwrap();

    let modes = [
        ("efficient", CompressionPerformance::Efficient),
        ("automatic", CompressionPerformance::Automatic),
        ("maximum", CompressionPerformance::Maximum),
    ];

    for (name, mode) in modes {
        let destination = root.join(format!("{name}.zip"));

        create_zip_with_performance(&[source_dir.clone()], &destination, mode).unwrap_or_else(
            |error| {
                panic!("{name} ZIP creation failed: {error:?}");
            },
        );

        assert!(destination.exists());
        assert!(fs::metadata(&destination).unwrap().len() > 0);

        let backend = ZipBackend;

        let entries = backend.list(&destination).unwrap_or_else(|error| {
            panic!("{name} ZIP list failed: {error:?}");
        });

        assert!(entries.iter().any(|entry| entry.name == "source/hello.txt"));

        assert!(
            entries
                .iter()
                .any(|entry| entry.name == "source/folder/nested.txt")
        );

        let extract_dir = root.join(format!("extracted-{name}"));

        backend
            .extract(&destination, &extract_dir)
            .unwrap_or_else(|error| {
                panic!("{name} ZIP extraction failed: {error:?}");
            });

        assert_eq!(
            fs::read_to_string(extract_dir.join("source").join("hello.txt")).unwrap(),
            "Hello from Arkive performance test!"
        );

        assert_eq!(
            fs::read_to_string(extract_dir.join("source").join("folder").join("nested.txt"))
                .unwrap(),
            "Nested Arkive file"
        );
    }

    fs::remove_dir_all(root).unwrap();
}
#[test]
#[ignore]
fn benchmark_zip_performance() {
    use std::{fs, time::Instant};

    const TEST_SIZE: usize = 100 * 1024 * 1024; // 100 MiB

    let root = std::env::temp_dir().join("arkive-zip-benchmark");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let source = root.join("benchmark.bin");

    println!("Creating 100 MiB benchmark file...");

    // 압축 가능한 데이터를 사용합니다.
    // 단순 0 채우기보다는 반복 패턴을 사용해 실제 Deflate 동작을 유도합니다.
    let pattern = b"Arkive compression benchmark data 0123456789\n";
    let mut data = Vec::with_capacity(TEST_SIZE);

    while data.len() < TEST_SIZE {
        let remaining = TEST_SIZE - data.len();
        let amount = remaining.min(pattern.len());
        data.extend_from_slice(&pattern[..amount]);
    }

    fs::write(&source, data).unwrap();

    let modes = [
        ("Efficient", CompressionPerformance::Efficient),
        ("Automatic", CompressionPerformance::Automatic),
        ("Maximum", CompressionPerformance::Maximum),
    ];

    println!();
    println!("=== Arkive ZIP Benchmark ===");

    for (name, mode) in modes {
        let destination = root.join(format!("{}.zip", name.to_lowercase()));

        let started = Instant::now();

        create_zip_with_performance(&[source.clone()], &destination, mode).unwrap_or_else(
            |error| {
                panic!("{name} compression failed: {error:?}");
            },
        );

        let elapsed = started.elapsed();

        let compressed_size = fs::metadata(&destination).unwrap().len();

        let ratio = compressed_size as f64 / TEST_SIZE as f64 * 100.0;

        let throughput = (TEST_SIZE as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64();

        println!(
            "{name:10} | {:8.3}s | {:8.2} MiB/s | {:8.2} MiB | {:6.2}%",
            elapsed.as_secs_f64(),
            throughput,
            compressed_size as f64 / 1024.0 / 1024.0,
            ratio,
        );
    }

    println!("============================");

    let _ = fs::remove_dir_all(root);
}
#[test]
#[ignore]
fn benchmark_zip_multifile_performance() {
    use std::{fs, time::Instant};

    const FILE_COUNT: usize = 20;
    const FILE_SIZE: usize = 5 * 1024 * 1024; // 파일당 5 MiB
    const TOTAL_SIZE: usize = FILE_COUNT * FILE_SIZE; // 총 100 MiB

    let root = std::env::temp_dir().join("arkive-zip-multifile-benchmark");
    let _ = fs::remove_dir_all(&root);

    let source_dir = root.join("source");
    fs::create_dir_all(&source_dir).unwrap();

    println!(
        "Creating {} files × {} MiB = {} MiB...",
        FILE_COUNT,
        FILE_SIZE / 1024 / 1024,
        TOTAL_SIZE / 1024 / 1024
    );

    /*
     * 완전히 동일한 데이터만 반복하지 않고,
     * 파일마다 조금씩 다른 deterministic binary 데이터를 만듭니다.
     *
     * rand crate가 필요하지 않으므로 Cargo.toml 변경도 없습니다.
     */
    for file_index in 0..FILE_COUNT {
        let mut data = Vec::with_capacity(FILE_SIZE);

        let mut state = 0x1234_5678_9ABC_DEF0u64 ^ (file_index as u64);

        while data.len() < FILE_SIZE {
            // xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;

            let bytes = state.to_le_bytes();
            let remaining = FILE_SIZE - data.len();
            let amount = remaining.min(bytes.len());

            data.extend_from_slice(&bytes[..amount]);
        }

        fs::write(source_dir.join(format!("file-{file_index:03}.bin")), data).unwrap();
    }

    let modes = [
        ("Efficient", CompressionPerformance::Efficient),
        ("Automatic", CompressionPerformance::Automatic),
        ("Maximum", CompressionPerformance::Maximum),
    ];

    println!();
    println!("=== Arkive Multi-file ZIP Benchmark ===");

    for (name, mode) in modes {
        let destination = root.join(format!("multi-{}.zip", name.to_lowercase()));

        let started = Instant::now();

        create_zip_with_performance(&[source_dir.clone()], &destination, mode).unwrap_or_else(
            |error| {
                panic!("{name} compression failed: {error:?}");
            },
        );

        let elapsed = started.elapsed();

        let compressed_size = fs::metadata(&destination).unwrap().len();

        let throughput = (TOTAL_SIZE as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64();

        let ratio = compressed_size as f64 / TOTAL_SIZE as f64 * 100.0;

        println!(
            "{name:10} | {:8.3}s | {:8.2} MiB/s | {:8.2} MiB | {:6.2}%",
            elapsed.as_secs_f64(),
            throughput,
            compressed_size as f64 / 1024.0 / 1024.0,
            ratio,
        );
    }

    println!("========================================");

    let _ = fs::remove_dir_all(root);
}
#[test]
#[ignore]
fn benchmark_7zip_compression_levels() {
    use std::{fs, time::Instant};

    const FILE_COUNT: usize = 20;
    const FILE_SIZE: usize = 5 * 1024 * 1024;
    const TOTAL_SIZE: usize = FILE_COUNT * FILE_SIZE;

    let root = std::env::temp_dir().join("arkive-7zip-level-benchmark");
    let _ = fs::remove_dir_all(&root);

    let source_dir = root.join("source");
    fs::create_dir_all(&source_dir).unwrap();

    // 어느 정도 압축되지만 지나치게 단순하지 않은 데이터
    let patterns = [
        b"Arkive document data: user content and metadata.\n".as_slice(),
        b"Lorem ipsum dolor sit amet, consectetur adipiscing elit.\n".as_slice(),
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\n".as_slice(),
    ];

    for file_index in 0..FILE_COUNT {
        let mut data = Vec::with_capacity(FILE_SIZE);

        while data.len() < FILE_SIZE {
            let pattern = patterns[(data.len() / 4096 + file_index) % patterns.len()];

            let remaining = FILE_SIZE - data.len();
            let amount = remaining.min(pattern.len());

            data.extend_from_slice(&pattern[..amount]);
        }

        fs::write(
            source_dir.join(format!("document-{file_index:03}.dat")),
            data,
        )
        .unwrap();
    }

    let workers = worker_count(CompressionPerformance::Maximum);

    println!();
    println!("=== Arkive 7-Zip Level Benchmark ===");
    println!("workers={workers}");

    for level in [6, 7, 8, 9] {
        let destination = root.join(format!("level-{level}.zip"));

        let started = Instant::now();

        create_zip_7zip(&[source_dir.clone()], &destination, level, workers).unwrap();

        let elapsed = started.elapsed();
        let compressed_size = fs::metadata(&destination).unwrap().len();

        let throughput = (TOTAL_SIZE as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64();

        let ratio = compressed_size as f64 / TOTAL_SIZE as f64 * 100.0;

        println!(
            "Level {level} | {:8.3}s | {:8.2} MiB/s | {:8.2} MiB | {:6.2}%",
            elapsed.as_secs_f64(),
            throughput,
            compressed_size as f64 / 1024.0 / 1024.0,
            ratio,
        );
    }

    println!("=====================================");

    let _ = fs::remove_dir_all(root);
}

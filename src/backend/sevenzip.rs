use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    path::{Path, PathBuf},
};

use crate::error::ArchiveError;
use crate::model::ArchiveEntry;

unsafe extern "C" {
    fn arkive_7zip_create_7z(
        sources: *const *const c_char,
        source_count: u32,
        destination: *const c_char,
        compression_level: i32,
        thread_count: u32,
    ) -> i32;
    fn arkive_7zip_extract(
        archive_path: *const std::ffi::c_char,
        destination: *const std::ffi::c_char,
    ) -> i32;

    fn arkive_7zip_list(
        archive_path: *const std::ffi::c_char,
        json_out: *mut *mut std::ffi::c_char,
    ) -> i32;

    fn arkive_7zip_string_free(value: *mut std::ffi::c_char);
}

pub fn create_7z(
    sources: &[PathBuf],
    destination: &Path,
    compression_level: i32,
    thread_count: usize,
) -> Result<(), ArchiveError> {
    if sources.is_empty() {
        return Err(ArchiveError::InvalidArchive);
    }

    let source_strings: Vec<CString> = sources
        .iter()
        .map(|path| {
            CString::new(path.to_string_lossy().as_bytes())
                .map_err(|_| ArchiveError::InvalidArchive)
        })
        .collect::<Result<_, _>>()?;

    let source_pointers: Vec<*const c_char> = source_strings
        .iter()
        .map(|source| source.as_ptr())
        .collect();

    let destination = CString::new(destination.to_string_lossy().as_bytes())
        .map_err(|_| ArchiveError::InvalidArchive)?;

    let result = unsafe {
        arkive_7zip_create_7z(
            source_pointers.as_ptr(),
            source_pointers.len() as u32,
            destination.as_ptr(),
            compression_level,
            thread_count as u32,
        )
    };

    match result {
        0 => Ok(()),

        1 => {
            eprintln!("arkive_7zip_create_7z returned 1: invalid argument/input");
            Err(ArchiveError::InvalidArchive)
        }

        5 => {
            eprintln!("arkive_7zip_create_7z returned 5: output I/O error");
            Err(ArchiveError::Io(std::io::Error::other("7-Zip I/O error")))
        }

        100 => {
            eprintln!("arkive_7zip_create_7z returned 100: 7-Zip engine error");
            Err(ArchiveError::InvalidArchive)
        }

        code => {
            eprintln!("arkive_7zip_create_7z returned unknown code: {code}");
            Err(ArchiveError::InvalidArchive)
        }
    }
}
pub fn list_7z(path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let path = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| ArchiveError::InvalidArchive)?;

    let mut json_ptr: *mut std::ffi::c_char = std::ptr::null_mut();

    let result = unsafe { arkive_7zip_list(path.as_ptr(), &mut json_ptr) };

    if result != 0 {
        return Err(match result {
            3 => ArchiveError::InvalidArchive,
            5 => ArchiveError::Io(std::io::Error::other("Failed to open 7Z archive")),
            _ => ArchiveError::InvalidArchive,
        });
    }

    if json_ptr.is_null() {
        return Err(ArchiveError::InvalidArchive);
    }

    let json = unsafe { CStr::from_ptr(json_ptr).to_string_lossy().into_owned() };

    unsafe {
        arkive_7zip_string_free(json_ptr);
    }

    serde_json::from_str::<Vec<ArchiveEntry>>(&json).map_err(|_| ArchiveError::InvalidArchive)
}
pub fn extract_7z(source: &Path, destination: &Path) -> Result<(), ArchiveError> {
    let source = CString::new(source.to_string_lossy().as_bytes())
        .map_err(|_| ArchiveError::InvalidArchive)?;

    let destination = CString::new(destination.to_string_lossy().as_bytes())
        .map_err(|_| ArchiveError::InvalidArchive)?;

    let result = unsafe { arkive_7zip_extract(source.as_ptr(), destination.as_ptr()) };

    match result {
        0 => Ok(()),

        3 => Err(ArchiveError::InvalidArchive),

        5 => Err(ArchiveError::Io(std::io::Error::other(
            "Failed to extract 7Z archive",
        ))),

        1001 => Err(ArchiveError::UnsafePath),

        1002 => Err(ArchiveError::UnsafeRedirection),

        _ => Err(ArchiveError::InvalidArchive),
    }
}
pub struct SevenZipBackend;

impl crate::backend::ArchiveBackend for SevenZipBackend {
    fn list(&self, path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        list_7z(path)
    }

    fn extract(&self, source: &Path, destination: &Path) -> Result<(), ArchiveError> {
        extract_7z(source, destination)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn create_7z_archive() {
        let root = std::env::temp_dir().join("arkive_7z_test");

        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let source = root.join("hello.txt");
        let archive = root.join("test.7z");

        fs::write(&source, b"Hello from Arkive 7Z!").unwrap();

        create_7z(&[source], &archive, 6, 2).unwrap();

        assert!(archive.exists());

        let metadata = fs::metadata(&archive).unwrap();

        assert!(metadata.len() > 0, "7Z archive is empty");

        println!(
            "Created 7Z: {} ({} bytes)",
            archive.display(),
            metadata.len()
        );
    }

    #[test]
    fn create_7z_multiple_files() {
        let root = std::env::temp_dir().join("arkive_7z_multiple_test");

        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let file1 = root.join("hello.txt");
        let file2 = root.join("world.txt");
        let archive = root.join("multiple.7z");

        fs::write(&file1, b"Hello from Arkive!").unwrap();

        fs::write(&file2, b"7Z multiple file test!").unwrap();

        create_7z(&[file1, file2], &archive, 6, 2).unwrap();

        assert!(archive.exists());
        assert!(fs::metadata(&archive).unwrap().len() > 0);

        println!("Created multi-file 7Z: {}", archive.display());
    }

    #[test]
    fn create_7z_directory_tree() {
        let root = std::env::temp_dir().join("arkive_7z_directory_test");

        let _ = fs::remove_dir_all(&root);

        let source = root.join("source");
        let nested = source.join("nested");

        fs::create_dir_all(&nested).unwrap();

        fs::write(source.join("hello.txt"), b"Hello root!").unwrap();

        fs::write(source.join("data.bin"), vec![0xAB; 1024]).unwrap();

        fs::write(nested.join("nested.txt"), b"Hello nested directory!").unwrap();

        let archive = root.join("directory.7z");

        create_7z(&[source], &archive, 6, 2).unwrap();

        assert!(archive.exists());

        let metadata = fs::metadata(&archive).unwrap();

        assert!(metadata.len() > 0, "Directory 7Z archive is empty");

        println!(
            "Created directory 7Z: {} ({} bytes)",
            archive.display(),
            metadata.len()
        );
    }

    #[test]
    #[ignore = "performance test: creates a large temporary file"]
    fn create_7z_large_file_performance() {
        use std::io::{BufWriter, Write};
        use std::time::Instant;

        const FILE_SIZE: usize = 256 * 1024 * 1024; // 256 MiB
        const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB

        let root = std::env::temp_dir().join("arkive_7z_performance_test");

        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let source = root.join("large.bin");

        // 완전한 0 데이터보다는 반복 패턴을 사용해서
        // 실제 LZMA2 압축 작업이 어느 정도 발생하도록 합니다.
        let mut chunk = vec![0u8; CHUNK_SIZE];

        for (index, byte) in chunk.iter_mut().enumerate() {
            *byte = ((index * 31 + index / 7) % 251) as u8;
        }

        {
            let file = fs::File::create(&source).unwrap();
            let mut writer = BufWriter::new(file);

            for _ in 0..(FILE_SIZE / CHUNK_SIZE) {
                writer.write_all(&chunk).unwrap();
            }

            writer.flush().unwrap();
        }

        let cpu_count = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);

        println!("Logical CPUs: {cpu_count}");
        println!("Source size: {} MiB", FILE_SIZE / 1024 / 1024);

        let modes = [
            ("Efficient", 3, (cpu_count / 4).max(1)),
            ("Automatic", 6, cpu_count.saturating_sub(2).max(1)),
            ("Maximum", 9, cpu_count),
        ];

        for (name, level, threads) in modes {
            let archive = root.join(format!("{}.7z", name.to_lowercase()));

            let start = Instant::now();

            create_7z(std::slice::from_ref(&source), &archive, level, threads).unwrap();

            let elapsed = start.elapsed();

            let compressed_size = fs::metadata(&archive).unwrap().len();

            let ratio = compressed_size as f64 / FILE_SIZE as f64 * 100.0;

            println!();
            println!("=== {name} ===");
            println!("Level: {level}");
            println!("Threads requested: {threads}");
            println!("Time: {:.3} sec", elapsed.as_secs_f64());
            println!(
                "Archive size: {:.2} MiB",
                compressed_size as f64 / 1024.0 / 1024.0
            );
            println!("Ratio: {ratio:.2}%");

            assert!(compressed_size > 0);
        }

        println!();
        println!("Archives: {}", root.display());
    }
    #[test]
    fn list_created_7z_archive() {
        let temp = std::env::temp_dir().join("arkive_7z_list_test");

        let _ = std::fs::remove_dir_all(&temp);

        std::fs::create_dir_all(&temp).unwrap();

        let source = temp.join("hello.txt");

        std::fs::write(&source, b"Hello from Arkive 7Z").unwrap();

        let archive = temp.join("test.7z");

        create_7z(&[source], &archive, 6, 2).unwrap();

        let entries = list_7z(&archive).unwrap();

        println!("{:#?}", entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "hello.txt");

        assert!(!entries[0].is_directory);
        assert_eq!(entries[0].size, 20);

        let _ = std::fs::remove_dir_all(&temp);
    }
    #[test]
    fn extract_created_7z_archive() {
        let temp = std::env::temp_dir().join("arkive_7z_extract_test");

        let _ = std::fs::remove_dir_all(&temp);

        let source_dir = temp.join("source");

        let nested_dir = source_dir.join("nested");

        let output_dir = temp.join("output");

        std::fs::create_dir_all(&nested_dir).unwrap();

        std::fs::write(source_dir.join("hello.txt"), b"Hello from Arkive").unwrap();

        std::fs::write(nested_dir.join("nested.txt"), b"Arkive 7Z extraction works").unwrap();

        let archive = temp.join("test.7z");

        create_7z(&[source_dir.clone()], &archive, 6, 2).unwrap();

        extract_7z(&archive, &output_dir).unwrap();

        let extracted_root = output_dir.join("source");

        assert_eq!(
            std::fs::read(extracted_root.join("hello.txt")).unwrap(),
            b"Hello from Arkive"
        );

        assert_eq!(
            std::fs::read(extracted_root.join("nested").join("nested.txt")).unwrap(),
            b"Arkive 7Z extraction works"
        );

        let _ = std::fs::remove_dir_all(&temp);
    }
    #[test]
    fn reject_7z_symbolic_link() {
        use std::path::Path;

        let archive = Path::new("/tmp/arkive-link-test/symlink.7z");

        let destination = Path::new("/tmp/arkive-link-test/extracted");

        let _ = std::fs::remove_dir_all(destination);

        let result = extract_7z(archive, destination);

        println!("Symlink extraction result: {:?}", result);

        assert!(
            matches!(result, Err(ArchiveError::UnsafeRedirection)),
            "expected UnsafeRedirection, got {:?}",
            result
        );
    }
    #[test]
    fn reject_7z_destination_symlink_ancestor() {
        use std::path::Path;

        let archive = Path::new("/tmp/arkive-destination-link-test/archive.7z");

        let destination = Path::new("/tmp/arkive-destination-link-test/output");

        let outside = Path::new("/tmp/arkive-destination-link-test/outside");

        let _ = std::fs::remove_dir_all("/tmp/arkive-destination-link-test");

        std::fs::create_dir_all("/tmp/arkive-destination-link-test/source/folder").unwrap();

        std::fs::write(
            "/tmp/arkive-destination-link-test/source/folder/evil.txt",
            b"must not escape",
        )
        .unwrap();

        create_7z(
            &[Path::new("/tmp/arkive-destination-link-test/source/folder").to_path_buf()],
            archive,
            6,
            1,
        )
        .unwrap();

        std::fs::create_dir_all(destination).unwrap();
        std::fs::create_dir_all(outside).unwrap();

        std::os::unix::fs::symlink(outside, destination.join("folder")).unwrap();

        let result = extract_7z(archive, destination);

        println!("Destination symlink result: {:?}", result);

        assert!(
            matches!(result, Err(ArchiveError::UnsafeRedirection)),
            "expected UnsafeRedirection, got {:?}",
            result
        );

        assert!(
            !outside.join("evil.txt").exists(),
            "archive escaped extraction root"
        );
    }
}

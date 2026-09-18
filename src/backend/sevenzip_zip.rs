use std::{
    ffi::CString,
    os::raw::c_char,
    path::{Path, PathBuf},
};

use crate::error::ArchiveError;

unsafe extern "C" {
    fn arkive_7zip_create_zip(
        sources: *const *const c_char,
        source_count: u32,
        destination: *const c_char,
        compression_level: i32,
        thread_count: u32,
    ) -> i32;
}

pub fn create_zip_7zip(
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
        arkive_7zip_create_zip(
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
            eprintln!("7-Zip bridge returned: 1 (invalid argument)");
            Err(ArchiveError::InvalidArchive)
        }

        5 => {
            eprintln!("7-Zip bridge returned: 5 (I/O error)");
            Err(ArchiveError::Io(std::io::Error::other("7-Zip I/O error")))
        }

        6 => {
            eprintln!("7-Zip bridge returned: 6 (internal error)");
            Err(ArchiveError::InvalidArchive)
        }

        100 => {
            eprintln!("7-Zip bridge returned: 100 (compression error)");
            Err(ArchiveError::InvalidArchive)
        }

        code => {
            eprintln!("7-Zip bridge returned unknown code: {code}");
            Err(ArchiveError::InvalidArchive)
        }
    }
}
#[test]
fn create_zip_with_7zip_engine() {
    use crate::backend::ArchiveBackend;
    use std::fs;

    let root = std::env::temp_dir().join("arkive_7zip_test");

    let _ = fs::remove_dir_all(&root);

    fs::create_dir_all(&root).unwrap();

    let source = root.join("hello.txt");
    let destination = root.join("archive.zip");

    fs::write(&source, b"Hello from Arkive 7-Zip engine!").unwrap();

    create_zip_7zip(&[source.clone()], &destination, 6, 2).expect("7-Zip ZIP creation failed");

    assert!(destination.exists(), "archive.zip was not created");

    assert!(
        fs::metadata(&destination).unwrap().len() > 0,
        "archive.zip is empty"
    );

    // 기존 Arkive ZIP reader로 다시 읽어서 검증
    let backend = crate::backend::zip::ZipBackend;

    let entries = backend
        .list(&destination)
        .expect("Arkive could not read generated ZIP");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "hello.txt");
    assert!(!entries[0].is_directory);

    let extract_dir = root.join("extracted");

    fs::create_dir_all(&extract_dir).unwrap();

    backend
        .extract(&destination, &extract_dir)
        .expect("Arkive could not extract generated ZIP");

    let extracted = fs::read_to_string(extract_dir.join("hello.txt")).unwrap();

    assert_eq!(extracted, "Hello from Arkive 7-Zip engine!");

    let _ = fs::remove_dir_all(&root);
}

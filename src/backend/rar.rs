use std::{
    ffi::CStr,
    os::raw::{c_char, c_int, c_void},
    path::Path,
};

use crate::{backend::ArchiveBackend, error::ArchiveError, model::ArchiveEntry};

unsafe extern "C" {
    fn arkive_rar_list(
        path: *const c_char,
        callback: Option<
            unsafe extern "C" fn(
                name: *const c_char,
                size: u64,
                compressed_size: u64,
                is_directory: c_int,
                context: *mut c_void,
            ),
        >,
        context: *mut c_void,
    ) -> i32;
}

pub struct RarBackend;

unsafe extern "C" fn collect_entry(
    name: *const c_char,
    size: u64,
    compressed_size: u64,
    is_directory: c_int,
    context: *mut c_void,
) {
    if name.is_null() || context.is_null() {
        return;
    }

    let entries = unsafe { &mut *(context as *mut Vec<ArchiveEntry>) };

    let name = unsafe { CStr::from_ptr(name) }
        .to_string_lossy()
        .into_owned();

    entries.push(ArchiveEntry {
        name,
        size,
        compressed_size,
        is_directory: is_directory != 0,
    });
}

impl ArchiveBackend for RarBackend {
    fn list(&self, path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let path = path.to_str().ok_or(ArchiveError::InvalidArchive)?;

        let path = std::ffi::CString::new(path).map_err(|_| ArchiveError::InvalidArchive)?;

        let mut entries = Vec::new();

        let result = unsafe {
            arkive_rar_list(
                path.as_ptr(),
                Some(collect_entry),
                &mut entries as *mut _ as *mut c_void,
            )
        };

        if result != 0 {
            return Err(ArchiveError::InvalidArchive);
        }

        Ok(entries)
    }

    fn extract(&self, _source: &Path, _destination: &Path) -> Result<(), ArchiveError> {
        Err(ArchiveError::UnsupportedFormat)
    }
}
#[cfg(test)]
mod tests {
    use crate::list_archive;

    #[test]
    fn list_rar_archive() {
        let entries = list_archive("test-data/sample.rar").expect("RAR listing failed");

        for entry in &entries {
            println!(
                "{} | {} bytes | compressed={} | directory={}",
                entry.name, entry.size, entry.compressed_size, entry.is_directory
            );
        }

        assert!(!entries.is_empty());
    }
}

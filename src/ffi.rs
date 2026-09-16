use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    path::Path,
};

use crate::{detector::detect_archive_format, model::ArchiveFormat};

#[unsafe(no_mangle)]
pub extern "C" fn archive_detect_format(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    let path = unsafe { CStr::from_ptr(path) };

    let Ok(path) = path.to_str() else {
        return std::ptr::null_mut();
    };

    let result = match detect_archive_format(Path::new(path)) {
        Ok(format) => match format {
            ArchiveFormat::Zip => "ZIP",
            ArchiveFormat::SevenZip => "7Z",
            ArchiveFormat::Rar4 => "RAR4",
            ArchiveFormat::Rar5 => "RAR5",
            ArchiveFormat::Tar => "TAR",
            ArchiveFormat::Gzip => "GZIP",
            ArchiveFormat::Xz => "XZ",
            ArchiveFormat::Zstd => "ZSTD",
            ArchiveFormat::Unknown => "UNKNOWN",
        },

        Err(_) => "ERROR",
    };

    CString::new(result).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub extern "C" fn archive_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    unsafe {
        let _ = CString::from_raw(value);
    }
}

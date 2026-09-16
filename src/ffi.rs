use crate::list_archive;
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    path::Path,
};

use crate::extract_archive;

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

#[unsafe(no_mangle)]
pub extern "C" fn archive_list(path: *const c_char) -> *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    let path = unsafe { CStr::from_ptr(path) };

    let Ok(path) = path.to_str() else {
        return std::ptr::null_mut();
    };

    let Ok(entries) = list_archive(path) else {
        return std::ptr::null_mut();
    };

    let Ok(json) = serde_json::to_string(&entries) else {
        return std::ptr::null_mut();
    };

    let Ok(result) = CString::new(json) else {
        return std::ptr::null_mut();
    };

    result.into_raw()
}
#[unsafe(no_mangle)]

pub extern "C" fn archive_extract(source: *const c_char, destination: *const c_char) -> i32 {
    if source.is_null() || destination.is_null() {
        return 1;
    }

    let source = unsafe { CStr::from_ptr(source) };

    let destination = unsafe { CStr::from_ptr(destination) };

    let Ok(source) = source.to_str() else {
        return 2;
    };

    let Ok(destination) = destination.to_str() else {
        return 2;
    };

    match extract_archive(Path::new(source), Path::new(destination)) {
        Ok(_) => 0,

        Err(_) => 3,
    }
}

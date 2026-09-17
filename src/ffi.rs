use crate::list_archive;
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    path::Path,
};

use crate::extract_archive;

use crate::error::ArchiveError;

use crate::{extract_archive_with_password, list_archive_with_password};

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

        Err(error) => archive_error_code(error)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archive_list_with_password(
    path: *const c_char,
    password: *const c_char,
) -> *mut c_char {
    if path.is_null() || password.is_null() {
        return std::ptr::null_mut();
    }

    let path = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };

    let password = match unsafe { CStr::from_ptr(password) }.to_str() {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };

    let entries = match list_archive_with_password(path, password) {
        Ok(entries) => entries,
        Err(_) => return std::ptr::null_mut(),
    };

    let json = match serde_json::to_string(&entries) {
        Ok(json) => json,
        Err(_) => return std::ptr::null_mut(),
    };

    match CString::new(json) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archive_extract_with_password(
    source: *const c_char,
    destination: *const c_char,
    password: *const c_char,
) -> i32 {
    if source.is_null() || destination.is_null() || password.is_null() {
        return 1;
    }

    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(value) => value,
        Err(_) => return 2,
    };

    let destination = match unsafe { CStr::from_ptr(destination) }.to_str() {
        Ok(value) => value,
        Err(_) => return 2,
    };

    let password = match unsafe { CStr::from_ptr(password) }.to_str() {
        Ok(value) => value,
        Err(_) => return 2,
    };

    match extract_archive_with_password(source, destination, password) {
        Ok(()) => 0,

        Err(ArchiveError::PasswordRequired) => 22,

        Err(ArchiveError::BadPassword) => 24,

        Err(ArchiveError::UnsafePath) => 1001,

        Err(ArchiveError::UnsafeRedirection) => 1002,

        Err(_) => 3,
    }
}
fn archive_error_code(error: ArchiveError) -> i32 {
    match error {
        ArchiveError::PasswordRequired => 22,
        ArchiveError::BadPassword => 24,
        ArchiveError::UnsafePath => 1001,
        ArchiveError::UnsafeRedirection => 1002,

        ArchiveError::UnsupportedFormat => 4,
        ArchiveError::InvalidArchive => 3,

        ArchiveError::Io(_) => 5,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archive_list_v2(path: *const c_char, json_out: *mut *mut c_char) -> i32 {
    if path.is_null() || json_out.is_null() {
        return 1;
    }

    unsafe {
        *json_out = std::ptr::null_mut();
    }

    let path = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(value) => value,
        Err(_) => return 2,
    };

    let entries = match list_archive(path) {
        Ok(entries) => entries,

        Err(error) => {
            return archive_error_code(error);
        }
    };

    let json = match serde_json::to_string(&entries) {
        Ok(json) => json,
        Err(_) => return 6,
    };

    let json = match CString::new(json) {
        Ok(json) => json,
        Err(_) => return 6,
    };

    unsafe {
        *json_out = json.into_raw();
    }

    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archive_list_with_password_v2(
    path: *const c_char,
    password: *const c_char,
    json_out: *mut *mut c_char,
) -> i32 {
    if path.is_null() || password.is_null() || json_out.is_null() {
        return 1;
    }

    unsafe {
        *json_out = std::ptr::null_mut();
    }

    let path = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(value) => value,
        Err(_) => return 2,
    };

    let password = match unsafe { CStr::from_ptr(password) }.to_str() {
        Ok(value) => value,
        Err(_) => return 2,
    };

    let entries = match list_archive_with_password(path, password) {
        Ok(entries) => entries,

        Err(error) => {
            return archive_error_code(error);
        }
    };

    let json = match serde_json::to_string(&entries) {
        Ok(json) => json,
        Err(_) => return 6,
    };

    let json = match CString::new(json) {
        Ok(json) => json,
        Err(_) => return 6,
    };

    unsafe {
        *json_out = json.into_raw();
    }

    0
}

use std::{fs::File, io::Read, path::Path};

use crate::{error::ArchiveError, model::ArchiveFormat};

const HEADER_SIZE: usize = 512;

pub fn detect_archive_format<P>(path: P) -> Result<ArchiveFormat, ArchiveError>
where
    P: AsRef<Path>,
{
    let mut file = File::open(path)?;
    let mut buffer = [0u8; HEADER_SIZE];
    let bytes_read = file.read(&mut buffer)?;
    let data = &buffer[..bytes_read];
    Ok(detect_from_bytes(data))
}

pub fn detect_from_bytes(data: &[u8]) -> ArchiveFormat {
    //
    // RAR5
    // 52 61 72 21 1A 07 01 00
    //
    if data.starts_with(&[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00]) {
        return ArchiveFormat::Rar5;
    }

    //
    // RAR4
    // 52 61 72 21 1A 07 00
    //
    if data.starts_with(&[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00]) {
        return ArchiveFormat::Rar4;
    }

    //
    // 7-Zip
    // 37 7A BC AF 27 1C
    //
    if data.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C]) {
        return ArchiveFormat::SevenZip;
    }

    //
    // ZIP
    // Normal ZIP
    // 50 4B 03 04
    //
    if data.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
        return ArchiveFormat::Zip;
    }

    //
    // Empty ZIP
    // 50 4B 05 06
    //
    if data.starts_with(&[0x50, 0x4B, 0x05, 0x06]) {
        return ArchiveFormat::Zip;
    }

    //
    // Spanned ZIP
    //
    if data.starts_with(&[0x50, 0x4B, 0x07, 0x08]) {
        return ArchiveFormat::Zip;
    }

    //
    // GZIP
    //
    if data.starts_with(&[0x1F, 0x8B]) {
        return ArchiveFormat::Gzip;
    }

    //
    // XZ
    //
    if data.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]) {
        return ArchiveFormat::Xz;
    }

    //
    // Zstandard
    //
    if data.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        return ArchiveFormat::Zstd;
    }

    //
    // TAR
    //
    // POSIX tar header:
    // offset 257 => "ustar"
    //
    if data.len() >= 262 && &data[257..262] == b"ustar" {
        return ArchiveFormat::Tar;
    }

    ArchiveFormat::Unknown
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_rar5() {
        let data = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x01, 0x00];

        assert_eq!(detect_from_bytes(&data), ArchiveFormat::Rar5);
    }

    #[test]
    fn detect_rar4() {
        let data = [0x52, 0x61, 0x72, 0x21, 0x1A, 0x07, 0x00];

        assert_eq!(detect_from_bytes(&data), ArchiveFormat::Rar4);
    }

    #[test]
    fn detect_seven_zip() {
        let data = [0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C];

        assert_eq!(detect_from_bytes(&data), ArchiveFormat::SevenZip);
    }

    #[test]
    fn detect_zip() {
        let data = [0x50, 0x4B, 0x03, 0x04];

        assert_eq!(detect_from_bytes(&data), ArchiveFormat::Zip);
    }

    #[test]
    fn detect_gzip() {
        let data = [0x1F, 0x8B];

        assert_eq!(detect_from_bytes(&data), ArchiveFormat::Gzip);
    }

    #[test]
    fn detect_xz() {
        let data = [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00];

        assert_eq!(detect_from_bytes(&data), ArchiveFormat::Xz);
    }

    #[test]
    fn detect_zstd() {
        let data = [0x28, 0xB5, 0x2F, 0xFD];

        assert_eq!(detect_from_bytes(&data), ArchiveFormat::Zstd);
    }

    #[test]
    fn unknown_data() {
        let data = [0x01, 0x02, 0x03, 0x04];

        assert_eq!(detect_from_bytes(&data), ArchiveFormat::Unknown);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Rar4,
    Rar5,
    Tar,
    Gzip,
    Xz,
    Zstd,
    Unknown,
}

impl ArchiveFormat {
    pub fn display_name(self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "ZIP",
            ArchiveFormat::SevenZip => "7-Zip",
            ArchiveFormat::Rar4 => "RAR4",
            ArchiveFormat::Rar5 => "RAR5",
            ArchiveFormat::Tar => "TAR",
            ArchiveFormat::Gzip => "GZIP",
            ArchiveFormat::Xz => "XZ",
            ArchiveFormat::Zstd => "Zstandard",
            ArchiveFormat::Unknown => "Unknown",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub is_directory: bool,
}

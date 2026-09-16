use std::path::Path;

use archive_core::{extract_archive, list_archive};

#[test]
fn list_real_archive() {
    let entries = list_archive("test-data/sample.zip").unwrap();

    assert!(!entries.is_empty());

    for entry in entries {
        println!("{} | {} bytes", entry.name, entry.size);
    }
}

#[test]
fn extract_real_archive() {
    let destination = "test-data/integration-output";

    extract_archive("test-data/sample.zip", destination).unwrap();

    assert!(Path::new(destination).exists());
}

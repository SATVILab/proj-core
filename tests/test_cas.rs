use proj::cas::ingest_directory;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_cas_integration_deduplication() {
    let root = tempdir().unwrap();
    let project_root = root.path().join("proj");
    let cas_remote_root = root.path().join("cas");

    fs::create_dir_all(&project_root).unwrap();
    fs::create_dir_all(&cas_remote_root).unwrap();

    let source_dir = project_root.join("data").join("raw");
    fs::create_dir_all(&source_dir).unwrap();

    // Create initial files
    fs::write(source_dir.join("file1.txt"), "content A").unwrap();
    fs::write(source_dir.join("file2.txt"), "content B").unwrap();

    // Pass 1: Ingest version 1.0.0
    ingest_directory(
        &project_root,
        &cas_remote_root,
        "raw",
        &source_dir,
        "1.0.0"
    ).unwrap();

    // Check objects were created
    let objects_dir = cas_remote_root.join("objects");
    let mut object_count_v1 = 0;
    for entry in walkdir(&objects_dir) {
        if entry.is_file() {
            object_count_v1 += 1;
        }
    }
    assert_eq!(object_count_v1, 2, "Should have 2 objects after first ingestion");

    // Pass 2: Ingest version 2.0.0 (duplicate files)
    ingest_directory(
        &project_root,
        &cas_remote_root,
        "raw",
        &source_dir,
        "2.0.0"
    ).unwrap();

    // Check object count hasn't changed
    let mut object_count_v2 = 0;
    for entry in walkdir(&objects_dir) {
        if entry.is_file() {
            object_count_v2 += 1;
        }
    }
    assert_eq!(object_count_v2, 2, "Duplicate files should be skipped, still 2 objects");

    // Pass 3: Modify a file and ingest version 3.0.0
    fs::write(source_dir.join("file2.txt"), "content B modified").unwrap();

    ingest_directory(
        &project_root,
        &cas_remote_root,
        "raw",
        &source_dir,
        "3.0.0"
    ).unwrap();

    let mut object_count_v3 = 0;
    for entry in walkdir(&objects_dir) {
        if entry.is_file() {
            object_count_v3 += 1;
        }
    }
    assert_eq!(object_count_v3, 3, "Modified file should result in 1 new object");

    // Verify manifests were created
    let manifests_dir = cas_remote_root.join("manifests");
    let mut manifest_count = 0;
    for entry in walkdir(&manifests_dir) {
        if entry.is_file() && entry.extension().unwrap_or_default() == "json" {
            manifest_count += 1;
        }
    }
    // versions 1.0.0 and 2.0.0 have the SAME file contents, BUT the version string in the DirectoryManifest differs.
    // Therefore, the JSON payload differs, the BLAKE3 hash differs, so 3 manifest files are created!
    assert_eq!(manifest_count, 3, "Should have 3 unique manifest files");

    // Verify ledger has 3 entries
    let ledger_path = project_root.join(".projr").join("manifests.csv");
    let ledger_content = fs::read_to_string(&ledger_path).unwrap();
    let lines: Vec<&str> = ledger_content.lines().collect();
    assert_eq!(lines.len(), 3, "Ledger should have 3 lines appended");
}

fn walkdir(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                paths.extend(walkdir(&path));
            } else {
                paths.push(path);
            }
        }
    }
    paths
}

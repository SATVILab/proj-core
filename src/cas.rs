use serde::{Serialize, Deserialize};

/// Represents a single file within a hashed directory structure.
///
/// This entry contains the file's relative path and its BLAKE3 hash.
///
/// # Examples
///
/// ```rust
/// use proj::cas::FileEntry;
///
/// let entry = FileEntry {
///     path: "data/file.txt".to_string(),
///     hash: "a1b2c3d4e5f6...".to_string(),
/// };
/// assert_eq!(entry.path, "data/file.txt");
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileEntry {
    /// Path relative to the labeled directory root
    pub path: String,
    /// BLAKE3 hash of file contents
    pub hash: String,
}

/// Represents the hashed structural layout of a directory.
///
/// Includes the label, version, and a sorted list of all recursive file entries.
///
/// # Examples
///
/// ```rust
/// use proj::cas::{DirectoryManifest, FileEntry};
///
/// let manifest = DirectoryManifest {
///     label: "raw".to_string(),
///     version: "1.0.0".to_string(),
///     files: vec![
///         FileEntry {
///             path: "data/a.txt".to_string(),
///             hash: "hashA".to_string(),
///         },
///         FileEntry {
///             path: "data/b.txt".to_string(),
///             hash: "hashB".to_string(),
///         },
///     ],
/// };
/// assert_eq!(manifest.files.len(), 2);
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DirectoryManifest {
    /// The directory label (e.g. "raw_data")
    pub label: String,
    /// The version string associated with this ingestion
    pub version: String,
    /// Sorted list of file entries
    pub files: Vec<FileEntry>,
}

use camino::Utf8Path;
use anyhow::Result;
use std::fs::{self, File};
use std::io::{self, Write, Read};
use chrono::Utc;
use anyhow::Context;
use tempfile::NamedTempFile;

/// Computes the BLAKE3 hash of a file's contents.
///
/// # Arguments
/// * `path` - The path to the file to hash.
///
/// # Returns
/// A hexadecimal string of the BLAKE3 hash.
///
/// # Errors
/// Returns an `io::Error` if the file cannot be opened or read.
///
/// # Examples
///
/// ```rust
/// use std::io::Write;
/// use tempfile::NamedTempFile;
/// use proj::cas::hash_file;
///
/// let mut file = NamedTempFile::new().unwrap();
/// write!(file, "hello world").unwrap();
/// let path = camino::Utf8Path::from_path(file.path()).unwrap();
/// let hash = hash_file(path).unwrap();
/// // "hello world" BLAKE3 hash: d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24
/// assert_eq!(hash, "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24");
/// ```
pub fn hash_file(path: &Utf8Path) -> Result<String> {
    let mut file = File::open(path.as_std_path()).context("Failed to open file for hashing")?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 65536]; // 64 KB buffer
    loop {
        let n = file.read(&mut buffer).context("Failed to read file for hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Ingests a directory into the CAS remote storage and logs it to the project ledger.
///
/// # Arguments
/// * `project_root` - The root path of the active workspace.
/// * `cas_remote_root` - The target remote CAS storage directory.
/// * `label` - The label of the directory being ingested (e.g., "raw").
/// * `source_dir` - The path to the directory to ingest.
/// * `version` - The version string for this ingestion.
///
/// # Errors
/// Returns an `io::Error` if files cannot be read/written, or directories cannot be created.

use crate::yml::{ValidatedConfig, InspectStrategy};

/// Verifies the structural integrity of a configured remote.
///
/// # Arguments
/// * `remote_title` - The name of the remote to inspect.
/// * `validated` - The validated configuration containing remote data.
///
/// # Errors
/// Returns an `io::Error` if the remote isn't found, manifest lacks files, or an integrity constraint fails.
pub fn verify_remote_integrity(remote_title: &str, validated: &ValidatedConfig) -> Result<()> {
    let remotes = validated.remotes.local.as_ref().context("No remotes configured")?;
    let remote = remotes.get(remote_title).with_context(|| format!("Remote '{}' not found", remote_title))?;

    let manifests_dir = remote.path.join("manifests");
    let objects_dir = remote.path.join("objects");

    if !manifests_dir.exists() {
        return Ok(()); // Empty but structurally valid
    }

    for entry in fs::read_dir(manifests_dir.as_std_path()).context("Failed to read manifests directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let entry_path = camino::Utf8PathBuf::try_from(entry.path())
            .context("Encountered non-UTF-8 path in manifests directory")?;

        if entry_path.extension() != Some("json") {
            continue;
        }

        match remote.inspect {
            InspectStrategy::Manifest => {
                // If the strategy is Manifest, the mere existence of the JSON file implies integrity.
                // It was confirmed by the read_dir above.
                continue;
            }
            InspectStrategy::File => {
                // Read and check each file
                let content = fs::read_to_string(entry_path.as_std_path())
                    .context("Failed to read manifest file content")?;
                let manifest: DirectoryManifest = serde_json::from_str(&content)
                    .context("Failed to parse manifest file JSON")?;

                for file_entry in manifest.files {
                    if file_entry.hash.len() < 2 {
                         anyhow::bail!("Invalid hash found.");
                    }
                    let prefix = &file_entry.hash[0..2];
                    let suffix = &file_entry.hash[2..];
                    let expected_path = objects_dir.join(prefix).join(suffix);

                    if !expected_path.exists() {
                        anyhow::bail!("Missing file object for hash {}.", file_entry.hash);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn ingest_directory(
    project_root: &Utf8Path,
    cas_remote_root: &Utf8Path,
    label: &str,
    source_dir: &Utf8Path,
    version: &str,
) -> Result<()> {
    let mut files = Vec::new();

    // 1. Scan and compute hashes
    fn scan_dir(dir: &Utf8Path, source_root: &Utf8Path, files: &mut Vec<FileEntry>) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir.as_std_path()).context("Failed to read directory during scan")? {
                let entry = entry.context("Failed to read directory entry")?;
                let path = camino::Utf8PathBuf::try_from(entry.path())
                    .context("Encountered non-UTF-8 path during scan")?;

                if path.is_dir() {
                    scan_dir(&path, source_root, files)?;
                } else {
                    let hash = hash_file(&path)?;
                    // Calculate relative path
                    let rel_path = path.strip_prefix(source_root).context("Failed to strip prefix")?.to_string();

                    // Standardize path separators to forward slashes
                    let rel_path = rel_path.replace("\\", "/");

                    files.push(FileEntry { path: rel_path, hash });
                }
            }
        }
        Ok(())
    }

    scan_dir(source_dir, source_dir, &mut files)?;

    // Sort files by path for deterministic hashing
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let manifest = DirectoryManifest {
        label: label.to_string(),
        version: version.to_string(),
        files: files.clone(),
    };

    // Serialize manifest and compute directory hash
    let manifest_json = serde_json::to_string(&manifest).context("Failed to serialize manifest JSON")?;
    let directory_hash = blake3::hash(manifest_json.as_bytes()).to_hex().to_string();

    // 2. Ingest files into CAS remote
    let objects_dir = cas_remote_root.join("objects");
    fs::create_dir_all(objects_dir.as_std_path()).context("Failed to create objects directory")?;

    for entry in &manifest.files {
        let hash = &entry.hash;
        if hash.len() < 2 {
            continue; // Invalid hash
        }
        let prefix = &hash[0..2];
        let suffix = &hash[2..];

        let target_dir = objects_dir.join(prefix);
        fs::create_dir_all(target_dir.as_std_path()).context("Failed to create target prefix directory")?;

        let target_path = target_dir.join(suffix);

        if !target_path.exists() {
            let source_file_path = source_dir.join(&entry.path);

            // Atomic copy using tempfile in the same directory
            let mut temp_file = NamedTempFile::new_in(&target_dir).context("Failed to create temporary file")?;
            let mut src_file = File::open(source_file_path.as_std_path()).context("Failed to open source file")?;
            io::copy(&mut src_file, &mut temp_file).context("Failed to copy file contents")?;

            temp_file.persist(&target_path)
                .context("Failed to persist temporary file to target path")?;
        }
    }

    // 3. Export Manifests
    let manifests_dir = cas_remote_root.join("manifests");
    fs::create_dir_all(manifests_dir.as_std_path()).context("Failed to create manifests directory")?;

    let manifest_path = manifests_dir.join(format!("{}.json", directory_hash));
    fs::write(manifest_path.as_std_path(), manifest_json).context("Failed to write manifest file")?;

    // 4. Update Flat Project Index
    let projr_dir = project_root.join(".projr");
    fs::create_dir_all(projr_dir.as_std_path()).context("Failed to create projr directory")?;

    let ledger_path = projr_dir.join("manifests.csv");
    let mut ledger_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ledger_path)
        .context("Failed to open ledger file")?;

    let timestamp = Utc::now().to_rfc3339();
    writeln!(ledger_file, "{},{},{},{}", version, label, directory_hash, timestamp)
        .context("Failed to write to ledger file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use tempfile::tempdir;
    use std::fs;


    #[test]
    fn test_hash_file() {
        let dir = tempdir().unwrap();
        let file_path = Utf8PathBuf::try_from(dir.path().join("test.txt")).unwrap();
        fs::write(&file_path, "test data").unwrap();

        let hash = hash_file(&file_path).unwrap();
        // precalculated blake3 hash for "test data"
        assert_eq!(hash, "6a953581d60dbebc9749b56d2383277fb02b58d260b4ccf6f119108fa0f1d4ef");
    }

    #[test]
    fn test_ingest_directory_empty() {
        let root = tempdir().unwrap();
        let cas_remote = Utf8PathBuf::try_from(root.path().join("cas")).unwrap();
        let proj_root = Utf8PathBuf::try_from(root.path().join("proj")).unwrap();
        let source_dir = proj_root.join("data").join("raw");

        fs::create_dir_all(cas_remote.as_std_path()).unwrap();
        fs::create_dir_all(source_dir.as_std_path()).unwrap();

        ingest_directory(&proj_root, &cas_remote, "raw", &source_dir, "v0.0.1").unwrap();

        let manifests_dir = cas_remote.join("manifests");
        let manifest_files: Vec<_> = fs::read_dir(manifests_dir.as_std_path()).unwrap().map(|e| e.unwrap().path()).collect();
        assert_eq!(manifest_files.len(), 1);

        let manifest_content = fs::read_to_string(&manifest_files[0]).unwrap();
        let manifest: DirectoryManifest = serde_json::from_str(&manifest_content).unwrap();

        assert_eq!(manifest.label, "raw");
        assert_eq!(manifest.version, "v0.0.1");
        assert_eq!(manifest.files.len(), 0);

        let ledger_path = proj_root.join(".projr").join("manifests.csv");
        assert!(ledger_path.exists());
    }
}

//! Utilities for visionOS artifact directories and file operations.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::lib::errors::ArtifactError;

const ZIP_DIR_PERMISSIONS: u32 = 0o755;

/// Ensure a job directory such as `target/visionos-builds/<job_id>/` exists.
pub fn ensure_job_dir(base_dir: &Path, job_id: &Uuid) -> Result<PathBuf, ArtifactError> {
    fs::create_dir_all(base_dir).map_err(|source| ArtifactError::CreateDir {
        path: base_dir.to_path_buf(),
        source,
    })?;

    let job_dir = base_dir.join(job_id.to_string());
    fs::create_dir_all(&job_dir).map_err(|source| ArtifactError::CreateDir {
        path: job_dir.clone(),
        source,
    })?;
    Ok(job_dir)
}

/// Delete artifacts whose TTL has expired and return the removed paths.
pub fn cleanup_expired_entries(
    root: &Path,
    ttl: Duration,
    now: DateTime<Utc>,
) -> Result<Vec<PathBuf>, ArtifactError> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut removed = Vec::new();
    for entry in fs::read_dir(root).map_err(|source| ArtifactError::ReadDir {
        path: root.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| ArtifactError::ReadDir {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|source| ArtifactError::Io {
            path: path.clone(),
            source,
        })?;
        let modified = metadata.modified().map_err(|source| ArtifactError::Io {
            path: path.clone(),
            source,
        })?;
        let modified = DateTime::<Utc>::from(modified);
        if now - modified > ttl {
            if path.is_dir() {
                fs::remove_dir_all(&path).map_err(|source| ArtifactError::Cleanup {
                    path: path.clone(),
                    source,
                })?;
            } else {
                fs::remove_file(&path).map_err(|source| ArtifactError::Cleanup {
                    path: path.clone(),
                    source,
                })?;
            }
            removed.push(path);
        }
    }
    Ok(removed)
}

/// Return the SHA256 of any file as a hex string.
pub fn compute_sha256(path: &Path) -> Result<String, ArtifactError> {
    let mut file = File::open(path).map_err(|source| ArtifactError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(|source| ArtifactError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Zip a directory tree, preserving empty directories as entries.
pub fn zip_directory(source: &Path, destination: &Path) -> Result<(), ArtifactError> {
    if !source.is_dir() {
        return Err(ArtifactError::InvalidSource {
            path: source.to_path_buf(),
        });
    }

    let file = File::create(destination).map_err(|source_err| ArtifactError::Io {
        path: destination.to_path_buf(),
        source: source_err,
    })?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(ZIP_DIR_PERMISSIONS);

    add_directory_to_zip(source, source, &mut zip, options)?;

    zip.finish().map_err(|source| ArtifactError::Zip {
        path: destination.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn add_directory_to_zip(
    base: &Path,
    current: &Path,
    zip: &mut ZipWriter<File>,
    options: FileOptions,
) -> Result<(), ArtifactError> {
    let entries = fs::read_dir(current).map_err(|source| ArtifactError::ReadDir {
        path: current.to_path_buf(),
        source,
    })?;

    let mut is_empty = true;
    for entry in entries {
        let entry = entry.map_err(|source| ArtifactError::ReadDir {
            path: current.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        if path.is_dir() {
            is_empty = false;
            let dir_name = format!("{relative}/");
            zip.add_directory(dir_name, options)
                .map_err(|source| ArtifactError::Zip {
                    path: path.clone(),
                    source,
                })?;
            add_directory_to_zip(base, &path, zip, options)?;
        } else {
            is_empty = false;
            zip.start_file(relative, options)
                .map_err(|source| ArtifactError::Zip {
                    path: path.clone(),
                    source,
                })?;
            let mut file = File::open(&path).map_err(|source| ArtifactError::Io {
                path: path.clone(),
                source,
            })?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|source| ArtifactError::Io {
                    path: path.clone(),
                    source,
                })?;
            zip.write_all(&buffer).map_err(|source| ArtifactError::Io {
                path: path.clone(),
                source,
            })?;
        }
    }

    if is_empty {
        let relative = current
            .strip_prefix(base)
            .unwrap_or(current)
            .to_string_lossy()
            .replace('\\', "/");
        if !relative.is_empty() {
            let dir_name = format!("{relative}/");
            zip.add_directory(dir_name, options)
                .map_err(|source| ArtifactError::Zip {
                    path: current.to_path_buf(),
                    source,
                })?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Read};

    use chrono::{Duration, Utc};
    use tempfile::tempdir;
    use uuid::Uuid;
    use zip::ZipArchive;

    use super::*;

    #[test]
    fn ensure_job_dir_creates_nested_directory() {
        let temp = tempdir().expect("can create temp directory");
        let job_id = Uuid::new_v4();

        let job_dir = ensure_job_dir(temp.path(), &job_id).expect("can create job directory");

        assert!(job_dir.exists(), "job directory exists");
        assert!(
            job_dir.ends_with(job_id.to_string()),
            "job ID is included in the path"
        );
    }

    #[test]
    fn cleanup_expired_entries_removes_old_jobs() {
        let temp = tempdir().expect("can create temp directory");
        let old_job = temp.path().join("old-job");
        fs::create_dir_all(&old_job).expect("can create old job");

        let ttl = Duration::minutes(5);
        let now = Utc::now() + Duration::minutes(10);

        let removed = cleanup_expired_entries(temp.path(), ttl, now).expect("cleanup succeeds");

        assert_eq!(removed, vec![old_job]);
        assert!(
            !removed[0].exists(),
            "deleted job directory should not exist"
        );
    }

    #[test]
    fn compute_sha256_returns_expected_digest() {
        let temp = tempdir().expect("can create temp directory");
        let file_path = temp.path().join("payload.bin");
        fs::write(&file_path, b"visionos-artifact").expect("can write test payload");

        let digest = compute_sha256(&file_path).expect("should successfully compute hash");

        assert_eq!(
            digest,
            "d201111930d4050d61a3078b01a8f030b3f6fbc24864db91ce8e1923a5604e72"
        );
    }

    #[test]
    fn zip_directory_packs_all_files() {
        let temp = tempdir().expect("can create temp directory");
        let source = temp.path().join("source");
        let nested = source.join("nested");
        fs::create_dir_all(&nested).expect("can create source directory");
        fs::write(source.join("root.txt"), b"root").expect("can write root file");
        fs::write(nested.join("child.txt"), b"child").expect("can write file to subdirectory");

        let destination = temp.path().join("artifacts.zip");
        zip_directory(&source, &destination).expect("should successfully create zip");

        let archive_file = fs::File::open(&destination).expect("can open zip");
        let mut archive = ZipArchive::new(archive_file).expect("can extract zip");

        {
            let mut root_entry = archive.by_name("root.txt").expect("root entry exists");
            let mut root_contents = String::new();
            root_entry
                .read_to_string(&mut root_contents)
                .expect("can read root.txt");
            assert_eq!(root_contents, "root");
        }

        {
            let mut child_entry = archive
                .by_name("nested/child.txt")
                .expect("child entry exists");
            let mut child_contents = String::new();
            child_entry
                .read_to_string(&mut child_contents)
                .expect("can read child.txt");
            assert_eq!(child_contents, "child");
        }
    }
}

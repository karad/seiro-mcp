//! Utilities for visionOS artifact directories and file operations.

use std::{
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

use crate::lib::errors::ArtifactError;

/// Unix permission bits applied to generated ZIP entries.
const ZIP_DIR_PERMISSIONS: u32 = 0o755;
/// Environment variable name for Codex home override.
const CODEX_HOME_ENV: &str = "CODEX_HOME";
/// Environment variable name for user home directory.
const HOME_ENV: &str = "HOME";

/// Skill file payload to install into Codex skill directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundledSkillFile<'a> {
    /// Path relative to skill directory (for example: `SKILL.md`).
    pub relative_path: &'a str,
    /// UTF-8 content to write.
    pub content: &'a str,
}

/// File write status for `install_skill_files`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillInstallStatus {
    Planned,
    Installed,
    SkippedExisting,
}

/// Result summary for skill file installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInstallResult {
    pub status: SkillInstallStatus,
    pub written_files: Vec<String>,
}

/// Removal status for `remove_skill_directory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillRemoveStatus {
    Removed,
    NotFound,
}

/// Result summary for skill directory removal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRemoveResult {
    pub status: SkillRemoveStatus,
    pub removed_files: Vec<String>,
}

/// Resolve Codex skills root directory.
///
/// Resolution order:
/// 1. `$CODEX_HOME/.codex/skills` when `CODEX_HOME` is set.
/// 2. `$HOME/.codex/skills` otherwise.
pub fn resolve_codex_skills_root() -> Result<PathBuf, &'static str> {
    resolve_codex_skills_root_from(env::var_os(CODEX_HOME_ENV), env::var_os(HOME_ENV))
}

/// Resolve skills root from explicit environment values (testable helper).
fn resolve_codex_skills_root_from(
    codex_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, &'static str> {
    if let Some(codex_home) = codex_home {
        return Ok(PathBuf::from(codex_home).join(".codex").join("skills"));
    }

    if let Some(home) = home {
        return Ok(PathBuf::from(home).join(".codex").join("skills"));
    }

    Err("CODEX_HOME and HOME are both unset")
}

/// Resolve full installation directory for a named skill.
pub fn resolve_skill_install_dir(skill_name: &str) -> Result<PathBuf, &'static str> {
    Ok(resolve_codex_skills_root()?.join(skill_name))
}

/// Install bundled skill files into destination directory.
///
/// In dry-run mode this function does not mutate filesystem state.
/// Without `force`, existing files are preserved and no write happens.
pub fn install_skill_files(
    destination_dir: &Path,
    files: &[BundledSkillFile<'_>],
    force: bool,
    dry_run: bool,
) -> Result<SkillInstallResult, io::Error> {
    let written_files = files
        .iter()
        .map(|file| file.relative_path.to_string())
        .collect::<Vec<_>>();

    let has_existing = files
        .iter()
        .any(|file| destination_dir.join(file.relative_path).exists());
    if has_existing && !force {
        return Ok(SkillInstallResult {
            status: SkillInstallStatus::SkippedExisting,
            written_files,
        });
    }

    if dry_run {
        return Ok(SkillInstallResult {
            status: SkillInstallStatus::Planned,
            written_files,
        });
    }

    fs::create_dir_all(destination_dir)?;
    for file in files {
        let path = destination_dir.join(file.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, file.content.as_bytes())?;
    }

    Ok(SkillInstallResult {
        status: SkillInstallStatus::Installed,
        written_files,
    })
}

/// Remove skill directory and return removed relative files.
///
/// If the directory does not exist, returns `NotFound` without error.
pub fn remove_skill_directory(destination_dir: &Path) -> Result<SkillRemoveResult, io::Error> {
    if !destination_dir.exists() {
        return Ok(SkillRemoveResult {
            status: SkillRemoveStatus::NotFound,
            removed_files: Vec::new(),
        });
    }

    let mut removed_files = Vec::new();
    collect_relative_files(destination_dir, destination_dir, &mut removed_files)?;
    removed_files.sort();

    if destination_dir.is_dir() {
        fs::remove_dir_all(destination_dir)?;
    } else {
        fs::remove_file(destination_dir)?;
    }

    Ok(SkillRemoveResult {
        status: SkillRemoveStatus::Removed,
        removed_files,
    })
}

/// Recursively collect relative file paths under a base directory.
fn collect_relative_files(
    base: &Path,
    current: &Path,
    out: &mut Vec<String>,
) -> Result<(), io::Error> {
    if current.is_file() {
        let relative = current
            .strip_prefix(base)
            .unwrap_or(current)
            .to_string_lossy()
            .replace('\\', "/");
        if !relative.is_empty() {
            out.push(relative);
        }
        return Ok(());
    }

    if !current.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_relative_files(base, &path, out)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(relative);
        }
    }

    Ok(())
}

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

/// Walk and append directory entries into a ZIP archive.
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
    use std::{fs, io::Read, path::PathBuf};

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

    #[test]
    fn resolve_codex_skills_root_prefers_codex_home() {
        let root = resolve_codex_skills_root_from(
            Some("/tmp/codex-home".into()),
            Some("/tmp/home".into()),
        )
        .expect("resolution succeeds");

        assert_eq!(root, PathBuf::from("/tmp/codex-home/.codex/skills"));
    }

    #[test]
    fn resolve_codex_skills_root_falls_back_to_home() {
        let root = resolve_codex_skills_root_from(None, Some("/tmp/home".into()))
            .expect("resolution succeeds");

        assert_eq!(root, PathBuf::from("/tmp/home/.codex/skills"));
    }

    #[test]
    fn install_skill_files_dry_run_is_non_mutating() {
        let temp = tempdir().expect("can create temp directory");
        let destination = temp.path().join("skills").join("sample-skill");
        let files = [BundledSkillFile {
            relative_path: "SKILL.md",
            content: "sample",
        }];

        let result =
            install_skill_files(&destination, &files, false, true).expect("dry-run should succeed");

        assert_eq!(result.status, SkillInstallStatus::Planned);
        assert_eq!(result.written_files, vec!["SKILL.md".to_string()]);
        assert!(!destination.exists(), "dry-run must not create directory");
    }
}

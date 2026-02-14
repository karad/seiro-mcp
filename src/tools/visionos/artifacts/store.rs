use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Duration, Utc};
use tokio::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

use crate::lib::errors::ArtifactError;
use crate::lib::fs as artifact_fs;

pub const ARTIFACT_ROOT: &str = "target/visionos-builds";
const ARTIFACT_FALLBACK_ROOT: &str = "seiro-mcp/visionos-builds";

/// Build job status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildJobStatus {
    Succeeded,
    Failed,
}

/// Record of a build job.
#[derive(Debug, Clone)]
pub struct BuildJobRecord {
    pub job_id: Uuid,
    pub status: BuildJobStatus,
    pub artifact_zip: Option<PathBuf>,
    pub artifact_sha256: Option<String>,
    pub log_excerpt: String,
    pub finished_at: DateTime<Utc>,
}

/// Store that persists visionOS artifacts and enforces TTL.
#[derive(Clone, Debug)]
pub struct VisionOsArtifactStore {
    inner: Arc<VisionOsArtifactStoreInner>,
}

#[derive(Debug)]
struct VisionOsArtifactStoreInner {
    root: PathBuf,
    ttl: Duration,
    cleanup_interval: Duration,
    state: Mutex<ArtifactStoreState>,
}

#[derive(Debug)]
struct ArtifactStoreState {
    jobs: HashMap<Uuid, BuildJobRecord>,
    last_cleanup: Option<DateTime<Utc>>,
}

impl VisionOsArtifactStore {
    /// Build a store using the default artifact directory.
    pub fn new(ttl_secs: u32, cleanup_schedule_secs: u32) -> Self {
        let root = resolve_artifact_root();
        Self::with_root(root, ttl_secs, cleanup_schedule_secs)
    }

    /// Build a store with a custom root directory (useful for tests).
    pub fn with_root(root: PathBuf, ttl_secs: u32, cleanup_schedule_secs: u32) -> Self {
        Self {
            inner: Arc::new(VisionOsArtifactStoreInner {
                root,
                ttl: Duration::seconds(ttl_secs as i64),
                cleanup_interval: Duration::seconds(cleanup_schedule_secs as i64),
                state: Mutex::new(ArtifactStoreState {
                    jobs: HashMap::new(),
                    last_cleanup: None,
                }),
            }),
        }
    }

    /// Return the artifact root directory currently used by this store.
    pub fn root_dir(&self) -> PathBuf {
        self.inner.root.clone()
    }

    /// Record a successful job.
    pub async fn record_success(
        &self,
        job_id: Uuid,
        artifact_zip: PathBuf,
        artifact_sha256: String,
        log_excerpt: String,
        finished_at: DateTime<Utc>,
    ) -> Result<(), ArtifactError> {
        self.maybe_cleanup(finished_at).await;
        let mut state = self.inner.state.lock().await;
        state.jobs.insert(
            job_id,
            BuildJobRecord {
                job_id,
                status: BuildJobStatus::Succeeded,
                artifact_zip: Some(artifact_zip),
                artifact_sha256: Some(artifact_sha256),
                log_excerpt,
                finished_at,
            },
        );
        Ok(())
    }

    /// Record a failed job.
    pub async fn record_failure(
        &self,
        job_id: Uuid,
        log_excerpt: String,
        finished_at: DateTime<Utc>,
    ) -> Result<(), ArtifactError> {
        self.maybe_cleanup(finished_at).await;
        let mut state = self.inner.state.lock().await;
        state.jobs.insert(
            job_id,
            BuildJobRecord {
                job_id,
                status: BuildJobStatus::Failed,
                artifact_zip: None,
                artifact_sha256: None,
                log_excerpt,
                finished_at,
            },
        );
        Ok(())
    }

    pub(crate) async fn fetch_record(
        &self,
        job_id: &Uuid,
    ) -> Result<BuildJobRecord, crate::tools::visionos::artifacts::FetchBuildOutputError> {
        let now = Utc::now();
        self.maybe_cleanup(now).await;
        let mut state = self.inner.state.lock().await;
        let record = state.jobs.get(job_id).cloned().ok_or(
            crate::tools::visionos::artifacts::FetchBuildOutputError::JobNotFound {
                job_id: *job_id,
            },
        )?;
        if now - record.finished_at > self.inner.ttl {
            state.jobs.remove(job_id);
            return Err(
                crate::tools::visionos::artifacts::FetchBuildOutputError::ArtifactExpired {
                    job_id: *job_id,
                },
            );
        }
        Ok(record)
    }

    pub(crate) fn ttl_seconds_remaining(&self, record: &BuildJobRecord) -> u32 {
        let now = Utc::now();
        let expires_at = record.finished_at + self.inner.ttl;
        if expires_at <= now {
            return 0;
        }
        let remaining = expires_at - now;
        remaining.num_seconds().try_into().unwrap_or(0)
    }

    async fn maybe_cleanup(&self, now: DateTime<Utc>) {
        let should_cleanup = {
            let mut state = self.inner.state.lock().await;
            let should = state
                .last_cleanup
                .map(|last| now - last >= self.inner.cleanup_interval)
                .unwrap_or(true);
            if should {
                state.last_cleanup = Some(now);
            }
            should
        };

        if !should_cleanup {
            return;
        }

        if let Err(err) =
            artifact_fs::cleanup_expired_entries(&self.inner.root, self.inner.ttl, now)
        {
            warn!(
                target: "rmcp_sample::visionos",
                error = %err,
                root = %self.inner.root.display(),
                "Failed to clean artifact directory"
            );
        }

        let metadata_window = self.inner.ttl + self.inner.cleanup_interval;
        let mut state = self.inner.state.lock().await;
        state
            .jobs
            .retain(|_, record| now - record.finished_at <= metadata_window);
    }
}

fn resolve_artifact_root() -> PathBuf {
    let preferred = PathBuf::from(ARTIFACT_ROOT);
    let fallback = std::env::temp_dir().join(ARTIFACT_FALLBACK_ROOT);
    resolve_artifact_root_with(&preferred, &fallback)
}

fn resolve_artifact_root_with(preferred: &Path, fallback: &Path) -> PathBuf {
    if directory_writable(preferred) {
        return preferred.to_path_buf();
    }

    if directory_writable(fallback) {
        warn!(
            target: "rmcp_sample::visionos",
            preferred_root = %preferred.display(),
            fallback_root = %fallback.display(),
            "Artifact root is not writable; using temporary directory fallback"
        );
        return fallback.to_path_buf();
    }

    warn!(
        target: "rmcp_sample::visionos",
        preferred_root = %preferred.display(),
        fallback_root = %fallback.display(),
        "Artifact root and fallback are not writable; keeping preferred root"
    );
    preferred.to_path_buf()
}

fn directory_writable(path: &Path) -> bool {
    if fs::create_dir_all(path).is_err() {
        return false;
    }

    let probe = path.join(format!(
        ".seiro-mcp-write-probe-{}-{}",
        std::process::id(),
        Uuid::new_v4()
    ));
    match OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn resolve_prefers_target_when_writable() {
        let temp = tempdir().expect("temporary directory");
        let preferred = temp.path().join("target/visionos-builds");
        let fallback = temp.path().join("tmp-fallback");

        let selected = resolve_artifact_root_with(&preferred, &fallback);
        assert_eq!(selected, preferred);
    }

    #[test]
    fn resolve_uses_fallback_when_target_is_not_writable() {
        let temp = tempdir().expect("temporary directory");
        let blocker = temp.path().join("target");
        fs::write(&blocker, b"file-blocker").expect("write blocker file");
        let preferred = blocker.join("visionos-builds");
        let fallback = temp.path().join("tmp-fallback");

        let selected = resolve_artifact_root_with(&preferred, &fallback);
        assert_eq!(selected, fallback);
    }

    #[cfg(unix)]
    #[test]
    fn resolve_uses_fallback_when_target_directory_is_read_only() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().expect("temporary directory");
        let preferred = temp.path().join("target/visionos-builds");
        fs::create_dir_all(&preferred).expect("create preferred dir");
        let fallback = temp.path().join("tmp-fallback");

        let mut permissions = fs::metadata(&preferred).expect("metadata").permissions();
        permissions.set_mode(0o555);
        fs::set_permissions(&preferred, permissions).expect("set read-only permissions");

        let selected = resolve_artifact_root_with(&preferred, &fallback);
        assert_eq!(selected, fallback);
    }
}

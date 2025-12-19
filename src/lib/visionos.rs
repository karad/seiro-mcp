//! Shared helpers for visionOS tools.
use std::path::{Path, PathBuf};

/// Returns true if `path` is under any of the allowed base paths.
pub fn is_allowed_path(path: &Path, allowed: &[PathBuf]) -> bool {
    allowed.iter().any(|base| path.starts_with(base))
}

/// Merge stdout/stderr and take at most `limit` characters from the end.
pub fn collect_log_excerpt(stdout: &[u8], stderr: &[u8], limit: usize) -> String {
    let mut combined = Vec::with_capacity(stdout.len() + stderr.len());
    combined.extend_from_slice(stdout);
    combined.extend_from_slice(stderr);
    let text = String::from_utf8_lossy(&combined);
    if text.chars().count() <= limit {
        return text.to_string();
    }
    text.chars()
        .rev()
        .take(limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

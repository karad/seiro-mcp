//! Shared helpers for visionOS tools.
use std::{fs, path::Path, path::PathBuf};

/// Returns true if `path` is under any of the allowed base paths.
pub fn is_allowed_path(path: &Path, allowed: &[PathBuf]) -> bool {
    let Ok(path) = fs::canonicalize(path) else {
        return false;
    };
    allowed
        .iter()
        .filter_map(|base| fs::canonicalize(base).ok())
        .any(|base| path.starts_with(base))
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

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::tempdir;

    use super::*;

    #[cfg(unix)]
    fn symlink_dir(original: &Path, link: &Path) {
        std::os::unix::fs::symlink(original, link).expect("can create directory symlink");
    }

    #[test]
    fn allowed_path_rejects_parent_traversal_escape_after_canonicalization() {
        let temp = tempdir().expect("can create temp dir");
        let allowed = temp.path().join("allowed");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&allowed).expect("can create allowed dir");
        fs::create_dir_all(&outside).expect("can create outside dir");
        let escaped = allowed.join("..").join("outside");

        assert!(!is_allowed_path(&escaped, &[allowed]));
    }

    #[cfg(unix)]
    #[test]
    fn allowed_path_rejects_symlink_escape_after_canonicalization() {
        let temp = tempdir().expect("can create temp dir");
        let allowed = temp.path().join("allowed");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&allowed).expect("can create allowed dir");
        fs::create_dir_all(&outside).expect("can create outside dir");
        let link = allowed.join("linked-outside");
        symlink_dir(&outside, &link);

        assert!(!is_allowed_path(&link, &[allowed]));
    }

    #[test]
    fn allowed_path_accepts_child_after_canonicalization() {
        let temp = tempdir().expect("can create temp dir");
        let allowed = temp.path().join("allowed");
        let child = allowed.join("child");
        fs::create_dir_all(&child).expect("can create child dir");

        assert!(is_allowed_path(&child, &[allowed]));
    }
}

//! Shared helpers reused across modules (e.g., path validation).

use std::path::Path;

/// Returns true if the path is non-empty and absolute.
pub fn is_nonempty_absolute(path: &Path) -> bool {
    !path.as_os_str().is_empty() && path.is_absolute()
}

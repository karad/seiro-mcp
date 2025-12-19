use std::env;
use std::path::{Path, PathBuf};

pub fn repo_root() -> anyhow::Result<PathBuf> {
    let mut dir = env::current_dir()?;
    loop {
        if looks_like_repo_root(&dir) {
            return Ok(dir);
        }
        if !dir.pop() {
            anyhow::bail!("failed to find repository root (no Cargo.toml/.git found)");
        }
    }
}

fn looks_like_repo_root(dir: &Path) -> bool {
    dir.join("Cargo.toml").is_file() || dir.join(".git").is_dir()
}

pub fn rel_from(root: &Path, path: &Path) -> PathBuf {
    match path.strip_prefix(root) {
        Ok(p) => p.to_path_buf(),
        Err(_) => path.to_path_buf(),
    }
}

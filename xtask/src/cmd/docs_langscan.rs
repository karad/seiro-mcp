use crate::fs;
use crate::repo;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run(path: Option<PathBuf>) -> Result<()> {
    let root = repo::repo_root()?;
    let scan_root = path.unwrap_or_else(|| PathBuf::from("docs"));
    let scan_root = if scan_root.is_absolute() {
        scan_root
    } else {
        root.join(scan_root)
    };

    let files = fs::walk_files(&scan_root, |dir| should_skip_dir(&scan_root, dir))?;
    let mut hits = Vec::new();
    for file in files {
        if should_skip_file(&file) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&file) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            if contains_japanese(line) {
                let rel = repo::rel_from(&root, &file);
                hits.push(format!("{}:{}:{}", rel.display(), idx + 1, line.trim_end()));
            }
        }
    }

    if hits.is_empty() {
        println!("No Japanese text detected in docs/.");
        return Ok(());
    }

    println!("{}", hits.join("\n"));
    anyhow::bail!(
        "Japanese text detected in docs/. Please translate or add to allowed exceptions."
    );
}

fn should_skip_dir(scan_root: &Path, dir: &Path) -> bool {
    // We intentionally allow Japanese docs under docs/ja/.
    let Ok(rel) = dir.strip_prefix(scan_root) else {
        return false;
    };
    rel.components()
        .next()
        .is_some_and(|c| c.as_os_str() == std::ffi::OsStr::new("ja"))
}

fn should_skip_file(path: &Path) -> bool {
    // Match the original script behavior: scan everything under docs/.
    // Avoid large/binary files by skipping non-utf8 at read-time.
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == ".DS_Store")
        .unwrap_or(false)
}

fn contains_japanese(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c,
          '\u{3040}'..='\u{309F}'
          | '\u{30A0}'..='\u{30FF}'
          | '\u{3400}'..='\u{4DBF}'
          | '\u{4E00}'..='\u{9FFF}'
          | '\u{FF65}'..='\u{FF9F}'
        )
    })
}

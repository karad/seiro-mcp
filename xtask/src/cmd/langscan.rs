use crate::fs;
use crate::repo;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run(path: Option<PathBuf>) -> Result<()> {
    let root = repo::repo_root()?;
    let scan_root = path.unwrap_or_else(|| root.clone());
    let scan_root = if scan_root.is_absolute() {
        scan_root
    } else {
        root.join(scan_root)
    };

    let files = fs::walk_files(&scan_root, |dir| should_skip_dir(&root, dir))?;
    let mut hits = Vec::new();
    for file in files {
        if should_skip_file(&root, &file) {
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
        println!("No Japanese text detected outside excluded paths.");
        return Ok(());
    }

    println!("{}", hits.join("\n"));
    anyhow::bail!(
        "Japanese text detected outside excluded paths. Please translate or move to allowed paths."
    );
}

fn should_skip_dir(repo_root: &Path, dir: &Path) -> bool {
    let rel = repo::rel_from(repo_root, dir);
    rel.components().any(|c| {
        let c = c.as_os_str();
        c == ".git"
            || c == "target"
            || c == "specs"
            || c == ".specify"
            || c == "docs"
            || c == ".codex"
    })
}

fn should_skip_file(repo_root: &Path, path: &Path) -> bool {
    let rel = repo::rel_from(repo_root, path);
    rel.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "AGENTS.md")
        .unwrap_or(false)
}

fn contains_japanese(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c,
          '\u{3040}'..='\u{309F}' // Hiragana
          | '\u{30A0}'..='\u{30FF}' // Katakana
          | '\u{3400}'..='\u{4DBF}' // CJK Unified Ideographs Extension A
          | '\u{4E00}'..='\u{9FFF}' // CJK Unified Ideographs
          | '\u{FF65}'..='\u{FF9F}' // Halfwidth Katakana
        )
    })
}

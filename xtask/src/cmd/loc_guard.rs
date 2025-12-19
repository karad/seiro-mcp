use crate::repo;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn run(baseline: Option<PathBuf>) -> Result<()> {
    let root = repo::repo_root()?;
    let baseline_path =
        baseline.unwrap_or_else(|| PathBuf::from("specs/008-src-refactor/loc-baseline.txt"));
    let baseline_path = if baseline_path.is_absolute() {
        baseline_path
    } else {
        root.join(baseline_path)
    };

    if !baseline_path.is_file() {
        anyhow::bail!(
            "baseline file not found: {}",
            repo::rel_from(&root, &baseline_path).display()
        );
    }

    let current_top5 = current_top5(&root)?;
    println!("Current top 5 longest Rust files:");
    for (count, rel) in &current_top5 {
        println!("{count} {}", rel.display());
    }

    let mut violations = 0usize;

    for (count, rel) in &current_top5 {
        if *count > 300 {
            eprintln!("FAIL: {} has {} lines (>300)", rel.display(), count);
            violations += 1;
        }
    }

    let current_map: HashMap<PathBuf, usize> =
        current_top5.iter().map(|(c, p)| (p.clone(), *c)).collect();

    let baseline_text = std::fs::read_to_string(&baseline_path)?;
    for line in baseline_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(base_count_str) = parts.next() else {
            continue;
        };
        let Some(base_file_str) = parts.next() else {
            continue;
        };
        let Ok(base_count) = base_count_str.parse::<usize>() else {
            continue;
        };
        let base_file = PathBuf::from(base_file_str);

        let Some(current_count) = current_map.get(&base_file) else {
            eprintln!(
                "WARN: baseline file {} not in current top5 (ok)",
                base_file.display()
            );
            continue;
        };

        let target = (base_count * 7) / 10;
        if *current_count > target {
            eprintln!(
                "FAIL: {} has {} lines; need <= {} (30% reduction from {})",
                base_file.display(),
                current_count,
                target,
                base_count
            );
            violations += 1;
        }
    }

    if violations > 0 {
        anyhow::bail!("LOC guard failed");
    }

    println!("PASS: LOC guard satisfied (<=300 lines and >=30% reduction vs baseline).");
    Ok(())
}

fn current_top5(root: &Path) -> Result<Vec<(usize, PathBuf)>> {
    let src = root.join("src");
    let mut counts = Vec::new();
    collect_rs_files(root, &src, &mut counts)?;
    counts.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    Ok(counts.into_iter().take(5).collect())
}

fn collect_rs_files(root: &Path, dir: &Path, out: &mut Vec<(usize, PathBuf)>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ty = entry.file_type()?;
        if ty.is_dir() {
            collect_rs_files(root, &path, out)?;
            continue;
        }
        if ty.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let text = std::fs::read_to_string(&path)?;
            let count = text.lines().count();
            let rel = repo::rel_from(root, &path);
            out.push((count, rel));
        }
    }
    Ok(())
}

use crate::repo;
use anyhow::Result;
use std::path::Path;

pub fn run() -> Result<()> {
    let root = repo::repo_root()?;
    let src = root.join("src");
    let mut counts = Vec::new();
    collect_rs_files(&root, &src, &mut counts)?;

    counts.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let top = counts.into_iter().take(5).collect::<Vec<_>>();

    println!("Top 5 longest Rust files under src/ (line counts):");
    for (count, rel) in top {
        println!("{count} {}", rel.display());
    }

    Ok(())
}

fn collect_rs_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<(usize, std::path::PathBuf)>,
) -> Result<()> {
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

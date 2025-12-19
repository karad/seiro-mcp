use crate::repo;
use anyhow::Result;

pub fn run() -> Result<()> {
    let root = repo::repo_root()?;
    let required = [
        "docs/refactor/module-map.md",
        "docs/refactor/responsibility-guidelines.md",
    ];

    for rel in required {
        let path = root.join(rel);
        if !path.is_file() {
            anyhow::bail!("FAIL: missing {rel}");
        }
        let meta = std::fs::metadata(&path)?;
        if meta.len() == 0 {
            anyhow::bail!("FAIL: empty {rel}");
        }
    }

    println!("PASS: required refactor docs exist and are non-empty");
    Ok(())
}

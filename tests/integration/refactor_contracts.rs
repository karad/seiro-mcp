use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn discover_contract_json_paths() -> Result<Vec<PathBuf>> {
    let root = repo_root();
    let mut roots = Vec::new();
    let top_level = root.join("contracts");
    if top_level.is_dir() {
        roots.push(top_level);
    }

    let specs_root = root.join("specs");
    if specs_root.is_dir() {
        for entry in fs::read_dir(&specs_root).context("failed to read specs directory")? {
            let entry = entry.context("failed to read specs entry")?;
            let path = entry.path().join("contracts");
            if path.is_dir() {
                roots.push(path);
            }
        }
    }

    let mut json_paths = Vec::new();
    for contract_root in roots {
        collect_json_files(&contract_root, &mut json_paths)
            .with_context(|| format!("failed to scan {}", contract_root.display()))?;
    }

    json_paths.sort();
    Ok(json_paths)
}

fn collect_json_files(root: &PathBuf, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry.context("failed to read directory entry")?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path);
        }
    }
    Ok(())
}

fn sha256_hex(path: &PathBuf) -> Result<String> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read {path}", path = path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn fixture_path(relative: &str) -> PathBuf {
    repo_root().join(relative)
}

fn write_fixture(path: &PathBuf, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create fixture dir {}", parent.display()))?;
    }
    fs::write(path, contents.as_bytes())
        .with_context(|| format!("failed to write fixture {}", path.display()))?;
    Ok(())
}

#[test]
fn contracts_sha256_matches_baseline() -> Result<()> {
    let root = repo_root();
    let json_paths = discover_contract_json_paths()?;
    if json_paths.is_empty() {
        anyhow::bail!("No contracts/*.json found under contracts/ or specs/*/contracts");
    }

    let mut lines = Vec::new();
    for path in &json_paths {
        let hash = sha256_hex(path)?;
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(path.as_path())
            .to_string_lossy()
            .to_string();
        lines.push(format!("{hash}  {relative}"));
    }
    let actual = format!("{}\n", lines.join("\n"));

    let fixture = fixture_path("tests/fixtures/contracts_sha256.txt");
    if std::env::var("UPDATE_FIXTURES").ok().as_deref() == Some("1") {
        write_fixture(&fixture, &actual)?;
        return Ok(());
    }

    let expected = fs::read_to_string(&fixture)
        .with_context(|| format!("missing fixture {}", fixture.display()))?;
    assert_eq!(actual, expected);
    Ok(())
}

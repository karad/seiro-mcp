use crate::repo;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(out: Option<PathBuf>) -> Result<()> {
    let root = repo::repo_root()?;
    let out_path = out.unwrap_or_else(|| PathBuf::from("specs/008-src-refactor/api-baseline.txt"));
    let out_path = if out_path.is_absolute() {
        out_path
    } else {
        root.join(out_path)
    };

    eprintln!("Capturing contracts SHA256...");
    let contract_roots = find_contract_roots(&root)?;
    if contract_roots.is_empty() {
        anyhow::bail!("No contracts directories found under contracts/ or specs/*/contracts");
    }

    let mut hashes = Vec::new();
    for contract_root in contract_roots {
        for json in find_json_files(&contract_root)? {
            let rel = repo::rel_from(&root, &json);
            let hash = sha256_file(&json)?;
            hashes.push((rel, hash));
        }
    }
    hashes.sort_by(|a, b| a.0.cmp(&b.0));

    eprintln!("Capturing CLI --help...");
    let help = capture_help()?;

    let mut out_text = String::new();
    out_text.push_str("# contracts sha256\n");
    for (path, hash) in hashes {
        out_text.push_str(&format!("{hash}  {}\n", path.display()));
    }
    out_text.push('\n');
    out_text.push_str("# cargo run -- --help\n");
    out_text.push_str(&help);

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, out_text)?;
    println!(
        "Saved baseline to {}",
        repo::rel_from(&root, &out_path).display()
    );
    Ok(())
}

fn find_contract_roots(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let root_contracts = root.join("contracts");
    if root_contracts.is_dir() {
        out.push(root_contracts);
    }

    let specs = root.join("specs");
    if specs.is_dir() {
        for entry in std::fs::read_dir(specs)? {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let contracts = path.join("contracts");
            if contracts.is_dir() {
                out.push(contracts);
            }
        }
    }

    out.sort();
    out.dedup();
    Ok(out)
}

fn find_json_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_json_files(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ty = entry.file_type()?;
        if ty.is_dir() {
            collect_json_files(&path, out)?;
            continue;
        }
        if ty.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path);
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    Ok(format!("{digest:x}"))
}

fn capture_help() -> Result<String> {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("--help")
        .output()?;
    if !output.status.success() {
        anyhow::bail!("cargo run -- --help failed (status {})", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

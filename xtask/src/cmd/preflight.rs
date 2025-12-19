use crate::repo;
use anyhow::Result;
use std::process::{Command, Stdio};

pub fn run() -> Result<()> {
    let root = repo::repo_root()?;
    run_step(&root, "cargo fetch", &["fetch"])?;
    run_step(&root, "cargo check", &["check"])?;
    run_step(&root, "cargo test --all", &["test", "--all"])?;
    run_step(&root, "cargo fmt -- --check", &["fmt", "--", "--check"])?;
    run_step(
        &root,
        "cargo clippy -- -D warnings",
        &["clippy", "--", "-D", "warnings"],
    )?;
    run_step(&root, "cargo build --release", &["build", "--release"])?;
    Ok(())
}

fn run_step(root: &std::path::Path, label: &str, args: &[&str]) -> Result<()> {
    eprintln!("==> {label}");
    let status = Command::new("cargo")
        .args(args)
        .current_dir(root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        anyhow::bail!("{label} failed (status {status})");
    }
    Ok(())
}

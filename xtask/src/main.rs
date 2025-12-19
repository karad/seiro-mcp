mod cmd;
mod fs;
mod repo;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Repository maintenance tasks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the local quality gate (fetch/check/test/fmt/build).
    Preflight,
    /// Detect Japanese text outside excluded paths.
    Langscan {
        /// Optional path to scan (defaults to repository root)
        #[arg(value_name = "PATH")]
        path: Option<std::path::PathBuf>,
        /// Extra args (accepted for compatibility with scripts; currently ignored)
        #[arg(trailing_var_arg = true, value_name = "ARGS")]
        extra: Vec<String>,
    },
    /// Detect Japanese text under docs/.
    DocsLangscan {
        /// Optional path to scan (defaults to docs/)
        #[arg(value_name = "PATH")]
        path: Option<std::path::PathBuf>,
        /// Extra args (accepted for compatibility with scripts; currently ignored)
        #[arg(trailing_var_arg = true, value_name = "ARGS")]
        extra: Vec<String>,
    },
    /// Validate internal Markdown links and heading anchors under docs/.
    CheckDocsLinks {
        /// Markdown files to check (defaults to docs/*.md at depth 1)
        #[arg(value_name = "FILE")]
        files: Vec<std::path::PathBuf>,
    },
    /// Print top 5 longest Rust files under src/.
    LocBaseline,
    /// Enforce LOC ceiling and baseline reduction.
    LocGuard {
        /// Baseline file path (defaults to specs/008-src-refactor/loc-baseline.txt)
        #[arg(value_name = "BASELINE")]
        baseline: Option<std::path::PathBuf>,
    },
    /// Capture contracts sha256 and `cargo run -- --help` output.
    ApiBaseline {
        /// Output file path (defaults to specs/008-src-refactor/api-baseline.txt)
        #[arg(value_name = "OUT")]
        out: Option<std::path::PathBuf>,
    },
    /// Validate required refactor docs exist (Spec 008 helper).
    RefactorCheckDocs,
}

fn main() {
    if let Err(err) = real_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Preflight => {
            cmd::preflight::run()?;
        }
        Command::Langscan { path, extra: _ } => {
            cmd::langscan::run(path)?;
        }
        Command::DocsLangscan { path, extra: _ } => {
            cmd::docs_langscan::run(path)?;
        }
        Command::CheckDocsLinks { files } => {
            cmd::check_docs_links::run(files)?;
        }
        Command::LocBaseline => {
            cmd::loc_baseline::run()?;
        }
        Command::LocGuard { baseline } => {
            cmd::loc_guard::run(baseline)?;
        }
        Command::ApiBaseline { out } => {
            cmd::api_baseline::run(out)?;
        }
        Command::RefactorCheckDocs => {
            cmd::refactor_check_docs::run()?;
        }
    }
    Ok(())
}

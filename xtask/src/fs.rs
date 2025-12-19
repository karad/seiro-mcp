use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn walk_files(
    root: &Path,
    should_skip_dir: impl Fn(&Path) -> bool,
) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk_files_inner(root, &should_skip_dir, &mut out)?;
    Ok(out)
}

fn walk_files_inner(
    dir: &Path,
    should_skip_dir: &impl Fn(&Path) -> bool,
    out: &mut Vec<PathBuf>,
) -> io::Result<()> {
    if should_skip_dir(dir) {
        return Ok(());
    }

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let ty = entry.file_type()?;
        if ty.is_dir() {
            walk_files_inner(&path, should_skip_dir, out)?;
            continue;
        }
        if ty.is_file() {
            out.push(path);
        }
    }

    Ok(())
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("md"))
}

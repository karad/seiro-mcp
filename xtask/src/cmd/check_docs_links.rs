use crate::fs;
use crate::repo;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn run(files: Vec<PathBuf>) -> Result<()> {
    let root = repo::repo_root()?;
    let paths = if files.is_empty() {
        default_docs_files(&root)?
    } else {
        files
            .into_iter()
            .map(|p| if p.is_absolute() { p } else { root.join(p) })
            .collect()
    };

    let mut errors = Vec::new();
    let mut anchor_cache: HashMap<PathBuf, HashSet<String>> = HashMap::new();

    for path in paths {
        if !path.exists() {
            errors.push(format!(
                "{}: file not found (skipped)",
                repo::rel_from(&root, &path).display()
            ));
            continue;
        }

        let Ok(text) = std::fs::read_to_string(&path) else {
            errors.push(format!(
                "{}: failed to read (skipped)",
                repo::rel_from(&root, &path).display()
            ));
            continue;
        };

        let anchors = load_anchors(&mut anchor_cache, &path)?;
        for (line_no, link) in extract_links(&text) {
            if is_external_link(&link) {
                continue;
            }
            if let Some(anchor) = link.strip_prefix('#') {
                let slug = slugify(anchor);
                if !slug.is_empty() && !anchors.contains(&slug) {
                    errors.push(format!(
                        "{}:{}: missing anchor '#{}'",
                        repo::rel_from(&root, &path).display(),
                        line_no,
                        slug
                    ));
                }
                continue;
            }

            let (target_part, anchor_part) = split_link(&link);
            let target_path = resolve_target(&path, target_part);
            if !target_path.exists() {
                errors.push(format!(
                    "{}:{}: missing target file '{}'",
                    repo::rel_from(&root, &path).display(),
                    line_no,
                    target_part
                ));
                continue;
            }

            if let Some(anchor_part) = anchor_part {
                let target_anchors = load_anchors(&mut anchor_cache, &target_path)?;
                let slug = slugify(anchor_part);
                if !slug.is_empty() && !target_anchors.contains(&slug) {
                    errors.push(format!(
                        "{}:{}: missing anchor '#{}' in {}",
                        repo::rel_from(&root, &path).display(),
                        line_no,
                        slug,
                        repo::rel_from(&root, &target_path).display()
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        println!("All internal links and anchors OK.");
        return Ok(());
    }

    println!("Link check failed:");
    for err in errors {
        println!("  - {err}");
    }
    anyhow::bail!("internal link check failed");
}

fn default_docs_files(root: &Path) -> Result<Vec<PathBuf>> {
    let docs = root.join("docs");
    let mut out = Vec::new();
    if !docs.is_dir() {
        return Ok(out);
    }

    for entry in std::fs::read_dir(&docs)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && fs::is_markdown(&path) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn load_anchors(
    cache: &mut HashMap<PathBuf, HashSet<String>>,
    path: &Path,
) -> Result<HashSet<String>> {
    if let Some(existing) = cache.get(path) {
        return Ok(existing.clone());
    }

    let mut anchors = HashSet::new();
    if path.exists() {
        let Ok(text) = std::fs::read_to_string(path) else {
            cache.insert(path.to_path_buf(), anchors.clone());
            return Ok(anchors);
        };
        for line in text.lines() {
            let heading = line.strip_prefix('#');
            if let Some(_rest) = heading {
                let heading_text = line.trim_start_matches('#').trim();
                if !heading_text.is_empty() {
                    anchors.insert(slugify(heading_text));
                }
            }
        }
    }

    cache.insert(path.to_path_buf(), anchors.clone());
    Ok(anchors)
}

fn extract_links(text: &str) -> Vec<(usize, String)> {
    // Roughly matches: !?\[[^\]]*\]\(([^)]+)\)
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some((end_bracket, after)) = find_closing_bracket(bytes, i) {
                if after < bytes.len() && bytes[after] == b'(' {
                    if let Some(end_paren) = find_byte(bytes, b')', after + 1) {
                        let link =
                            String::from_utf8_lossy(&bytes[after + 1..end_paren]).to_string();
                        let line_no = 1 + text[..i].chars().filter(|&c| c == '\n').count();
                        out.push((line_no, link));
                        i = end_paren + 1;
                        continue;
                    }
                }
                i = end_bracket + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn find_closing_bracket(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    // Find `]` after `[`, then return (index_of_], index_after_]).
    let end = find_byte(bytes, b']', start + 1)?;
    Some((end, end + 1))
}

fn find_byte(bytes: &[u8], needle: u8, start: usize) -> Option<usize> {
    bytes
        .iter()
        .skip(start)
        .position(|&b| b == needle)
        .map(|p| start + p)
}

fn is_external_link(link: &str) -> bool {
    link.starts_with("http://")
        || link.starts_with("https://")
        || link.starts_with("mailto:")
        || link.starts_with("tel:")
}

fn split_link(link: &str) -> (&str, Option<&str>) {
    let mut parts = link.splitn(2, '#');
    let target = parts.next().unwrap_or("");
    let anchor = parts.next();
    (target, anchor)
}

fn resolve_target(from: &Path, target_part: &str) -> PathBuf {
    from.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(target_part)
}

fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in text.trim().to_lowercase().chars() {
        let keep = c.is_alphanumeric() || c == '_' || c == '-' || c.is_whitespace();
        if !keep {
            continue;
        }
        if c.is_whitespace() {
            if !out.is_empty() && !prev_dash {
                out.push('-');
                prev_dash = true;
            }
            continue;
        }
        if c == '-' {
            if !out.is_empty() && !prev_dash {
                out.push('-');
                prev_dash = true;
            }
            continue;
        }
        out.push(c);
        prev_dash = false;
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

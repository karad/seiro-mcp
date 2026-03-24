/// Parsed primary compiler error.
#[derive(Debug, Clone)]
pub struct ParsedPrimaryError {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub headline: String,
    pub excerpt: String,
}

/// Parse the first actionable `error:` line from compiler output.
pub fn parse_primary_error(stderr: &str, stdout: &str) -> Option<ParsedPrimaryError> {
    let merged = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };

    for raw_line in merged.lines() {
        let line = raw_line.trim();
        if line.is_empty() || !line.contains("error:") {
            continue;
        }

        if let Some((left, message)) = line.split_once(": error: ") {
            let mut parts = left.rsplitn(3, ':');
            let column_text = parts.next();
            let line_text = parts.next();
            let file_text = parts.next();
            if let (Some(column_text), Some(line_text), Some(file_text)) =
                (column_text, line_text, file_text)
            {
                let parsed_line = line_text.trim().parse::<u32>().ok();
                let parsed_column = column_text.trim().parse::<u32>().ok();
                if parsed_line.is_some() && parsed_column.is_some() {
                    return Some(ParsedPrimaryError {
                        file: Some(file_text.to_string()),
                        line: parsed_line,
                        column: parsed_column,
                        headline: message.trim().to_string(),
                        excerpt: line.to_string(),
                    });
                }
            }

            return Some(ParsedPrimaryError {
                file: None,
                line: None,
                column: None,
                headline: message.trim().to_string(),
                excerpt: line.to_string(),
            });
        }

        if let Some(message) = line.strip_prefix("error:") {
            return Some(ParsedPrimaryError {
                file: None,
                line: None,
                column: None,
                headline: message.trim().to_string(),
                excerpt: line.to_string(),
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_primary_error_reads_swift_location() {
        let stderr = "/tmp/ContentView.swift:105:22: error: no member 'generateCylinder'";
        let parsed = parse_primary_error(stderr, "").expect("error should parse");
        assert_eq!(parsed.file.as_deref(), Some("/tmp/ContentView.swift"));
        assert_eq!(parsed.line, Some(105));
        assert_eq!(parsed.column, Some(22));
        assert_eq!(parsed.headline, "no member 'generateCylinder'");
    }

    #[test]
    fn parse_primary_error_falls_back_to_non_location_error() {
        let stderr = "error: linker command failed with exit code 1";
        let parsed = parse_primary_error(stderr, "").expect("error should parse");
        assert!(parsed.file.is_none());
        assert_eq!(parsed.headline, "linker command failed with exit code 1");
    }
}

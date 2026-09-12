//! File contents, normalized for display. Read-only: nothing here ever writes.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const TAB: &str = "    ";
/// How far in we look for a NUL before calling a file binary.
const SNIFF: usize = 8 * 1024;

pub struct Buffer {
    pub path: Option<PathBuf>,
    pub lines: Vec<String>,
}

impl Buffer {
    /// An empty scratch buffer, used when merl is opened on a directory.
    pub fn empty() -> Self {
        Self {
            path: None,
            lines: vec![String::new()],
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("{}", path.display()))?;
        Ok(Self::from_bytes(path.to_path_buf(), &bytes))
    }

    fn from_bytes(path: PathBuf, bytes: &[u8]) -> Self {
        if bytes[..bytes.len().min(SNIFF)].contains(&0) {
            return Self {
                path: Some(path),
                lines: vec!["binary file".to_string()],
            };
        }
        let text = String::from_utf8_lossy(bytes);
        let mut lines: Vec<String> = text
            .split('\n')
            .map(|l| l.trim_end_matches('\r').replace('\t', TAB))
            .collect();
        // A trailing newline is a terminator, not an empty last line.
        if lines.len() > 1 && lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self {
            path: Some(path),
            lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(bytes: &[u8]) -> Buffer {
        Buffer::from_bytes(PathBuf::from("x"), bytes)
    }

    #[test]
    fn normalizes_text() {
        assert_eq!(load(b"a\tb\r\nc\n").lines, vec!["a    b", "c"]);
    }

    #[test]
    fn empty_file_has_one_line() {
        assert_eq!(load(b"").lines, vec![""]);
        assert_eq!(load(b"\n").lines, vec![""]);
    }

    #[test]
    fn binary_is_one_line() {
        assert_eq!(load(b"ELF\0\x01\x02").lines, vec!["binary file"]);
    }

    #[test]
    fn invalid_utf8_is_lossy() {
        assert_eq!(load(b"a\xffb").lines, vec!["a\u{fffd}b"]);
    }
}

//! Content errors: every problem names the file and, when known, the line and
//! column, so a broken asset can be fixed without reading Rust (ADR-0005).

use std::fmt;

/// One problem found while loading or validating an asset.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub struct ContentError {
    /// Human-facing file path, e.g. `assets/data/palette.ron`.
    pub file: String,
    /// 1-based line, if known.
    pub line: Option<u32>,
    /// 1-based column, if known (only shown together with a line).
    pub column: Option<u32>,
    /// What is wrong, in plain words.
    pub message: String,
}

impl ContentError {
    /// An error about a whole file (no position).
    pub fn new(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: None,
            column: None,
            message: message.into(),
        }
    }

    /// Attaches a 1-based line and optional column.
    #[must_use]
    pub fn at(mut self, line: u32, column: Option<u32>) -> Self {
        self.line = Some(line);
        self.column = column;
        self
    }
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file)?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }
        write!(f, ": {}", self.message)
    }
}

/// Every problem found in one load, reported together.
#[derive(Debug, Clone, PartialEq, Eq, Default, thiserror::Error)]
pub struct ContentErrors(pub Vec<ContentError>);

impl fmt::Display for ContentErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0.len();
        write!(f, "{n} content error{}:", if n == 1 { "" } else { "s" })?;
        for e in &self.0 {
            write!(f, "\n  {e}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_with_line_and_column() {
        let e = ContentError::new("assets/data/palette.ron", "unknown colour \"plyer\"")
            .at(12, Some(5));
        assert_eq!(
            e.to_string(),
            "assets/data/palette.ron:12:5: unknown colour \"plyer\""
        );
    }

    #[test]
    fn display_with_line_only() {
        let e = ContentError::new("a.ron", "bad").at(3, None);
        assert_eq!(e.to_string(), "a.ron:3: bad");
    }

    #[test]
    fn display_without_position() {
        let e = ContentError::new("a.ron", "missing file");
        assert_eq!(e.to_string(), "a.ron: missing file");
    }

    #[test]
    fn display_lists_all_errors() {
        let errs = ContentErrors(vec![
            ContentError::new("a.ron", "one").at(1, Some(2)),
            ContentError::new("b.ron", "two"),
        ]);
        assert_eq!(
            errs.to_string(),
            "2 content errors:\n  a.ron:1:2: one\n  b.ron: two"
        );
        let single = ContentErrors(vec![ContentError::new("a.ron", "one")]);
        assert_eq!(single.to_string(), "1 content error:\n  a.ron: one");
    }
}

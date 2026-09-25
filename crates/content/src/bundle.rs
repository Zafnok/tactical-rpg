//! The asset bundle: every file under the repo's `assets/` directory, embedded
//! in the binary at compile time (ADR-0005). This is the only way `content`
//! reads data; there is no filesystem I/O.

use include_dir::{Dir, include_dir};

static ASSETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../assets");

/// Prefix used when showing an asset path to a human (`assets/data/x.ron`).
pub const DISPLAY_ROOT: &str = "assets";

/// Returns the UTF-8 contents of the embedded file at `path` (relative to
/// `assets/`, `/`-separated), or `None` if it is missing or not valid UTF-8.
pub fn file(path: &str) -> Option<&'static str> {
    ASSETS.get_file(path)?.contents_utf8()
}

/// Lists the paths (relative to `assets/`, `/`-separated, sorted) of every
/// file directly inside directory `dir`. A missing directory yields an empty
/// list. Use `""` for the bundle root.
pub fn files_in(dir: &str) -> Vec<&'static str> {
    let entries = if dir.is_empty() {
        Some(&ASSETS)
    } else {
        ASSETS.get_dir(dir)
    };
    let mut paths: Vec<&'static str> = entries
        .into_iter()
        .flat_map(Dir::files)
        .filter_map(|f| f.path().to_str())
        .collect();
    paths.sort_unstable();
    paths
}

/// The human-facing name of an asset path, e.g. `assets/data/palette.ron`.
pub fn display_path(path: &str) -> String {
    format!("{DISPLAY_ROOT}/{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_existing_file() {
        let text = file("data/palette.ron");
        assert!(text.is_some_and(|t| t.contains("player")));
    }

    #[test]
    fn missing_file_is_none() {
        assert_eq!(file("data/does_not_exist.ron"), None);
    }

    #[test]
    fn lists_files_in_directory() {
        let files = files_in("data");
        assert!(files.contains(&"data/palette.ron"));
        assert!(files.iter().all(|p| p.starts_with("data/")));
        let mut sorted = files.clone();
        sorted.sort_unstable();
        assert_eq!(files, sorted);
    }

    #[test]
    fn root_and_missing_directories() {
        assert!(files_in("").iter().all(|p| !p.contains('/')));
        assert!(files_in("no_such_dir").is_empty());
    }

    #[test]
    fn display_path_prefixes_assets() {
        assert_eq!(display_path("data/palette.ron"), "assets/data/palette.ron");
    }
}

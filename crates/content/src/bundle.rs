//! The asset bundle: every file under the repo's `assets/` directory, embedded
//! in the binary at compile time (ADR-0005). This is the only way `content`
//! reads data; there is no filesystem I/O.
//!
//! With the `private-assets` feature the bought art in `assets-private/game/`
//! is embedded too and laid over `assets/` (ADR-0040): a private file
//! replaces the public file at the same path, and a private-only file is
//! added. Tests and gates build without the feature, so they always read the
//! public placeholders.

use include_dir::{Dir, File, include_dir};

static ASSETS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../assets");

#[cfg(feature = "private-assets")]
static PRIVATE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../assets-private/game");

/// Prefix used when showing an asset path to a human (`assets/data/x.ron`).
pub const DISPLAY_ROOT: &str = "assets";

/// The same prefix for a file that comes from the private assets
/// (`assets-private/game/portraits/x.png`).
pub const PRIVATE_DISPLAY_ROOT: &str = "assets-private/game";

/// One embedded directory.
struct Layer {
    /// The prefix its files are shown with.
    display_root: &'static str,
    /// Its files.
    dir: &'static Dir<'static>,
}

/// The embedded directories. A file is read from the first one that has it.
static LAYERS: &[Layer] = &[
    #[cfg(feature = "private-assets")]
    Layer {
        display_root: PRIVATE_DISPLAY_ROOT,
        dir: &PRIVATE,
    },
    Layer {
        display_root: DISPLAY_ROOT,
        dir: &ASSETS,
    },
];

/// The file at `path` and the layer it is read from: the first layer that
/// has it.
fn find(layers: &'static [Layer], path: &str) -> Option<(&'static Layer, &'static File<'static>)> {
    layers
        .iter()
        .find_map(|layer| Some((layer, layer.dir.get_file(path)?)))
}

/// The paths (sorted, each once) of the files of every layer inside `dir`:
/// directly inside it, or with `deep` also in any directory below it.
fn list(layers: &'static [Layer], dir: &str, deep: bool) -> Vec<&'static str> {
    let mut paths = Vec::new();
    for layer in layers {
        let root = if dir.is_empty() {
            Some(layer.dir)
        } else {
            layer.dir.get_dir(dir)
        };
        let mut pending: Vec<&'static Dir<'static>> = root.into_iter().collect();
        while let Some(dir) = pending.pop() {
            paths.extend(dir.files().filter_map(|f| f.path().to_str()));
            if deep {
                pending.extend(dir.dirs());
            }
        }
    }
    paths.sort_unstable();
    paths.dedup();
    paths
}

/// The human-facing name of `path` in `layers`: under the root of the layer
/// it is read from, or under `assets/` when no layer has it.
fn display_in(layers: &'static [Layer], path: &str) -> String {
    let root = find(layers, path).map_or(DISPLAY_ROOT, |(layer, _)| layer.display_root);
    format!("{root}/{path}")
}

/// Returns the UTF-8 contents of the embedded file at `path` (relative to
/// `assets/`, `/`-separated), or `None` if it is missing or not valid UTF-8.
pub fn file(path: &str) -> Option<&'static str> {
    find(LAYERS, path)?.1.contents_utf8()
}

/// Returns the raw bytes of the embedded file at `path` (relative to
/// `assets/`, `/`-separated), e.g. an image, or `None` if it is missing.
pub fn bytes(path: &str) -> Option<&'static [u8]> {
    find(LAYERS, path).map(|(_, file)| file.contents())
}

/// Lists the paths (relative to `assets/`, `/`-separated, sorted) of every
/// file directly inside directory `dir`. A missing directory yields an empty
/// list. Use `""` for the bundle root.
pub fn files_in(dir: &str) -> Vec<&'static str> {
    list(LAYERS, dir, false)
}

/// Lists the paths (relative to `assets/`, `/`-separated, sorted) of every
/// file inside directory `dir` or any directory below it. A missing
/// directory yields an empty list. Use `""` for the whole bundle.
pub fn files_under(dir: &str) -> Vec<&'static str> {
    list(LAYERS, dir, true)
}

/// The human-facing name of an asset path, e.g. `assets/data/palette.ron`
/// (or `assets-private/game/...` for a file read from the private assets).
pub fn display_path(path: &str) -> String {
    display_in(LAYERS, path)
}

#[cfg(test)]
mod tests {
    use include_dir::DirEntry;

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
    fn reads_binary_file() {
        let png = bytes("fonts/atlas.png").unwrap_or_default();
        assert!(png.starts_with(b"\x89PNG"));
        assert_eq!(bytes("fonts/nope.png"), None);
        assert_eq!(
            bytes("data/palette.ron").map(<[u8]>::len),
            file("data/palette.ron").map(str::len)
        );
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
    fn lists_files_under_a_directory_and_below() {
        let all = files_under("");
        for path in ["data/palette.ron", "fonts/atlas.png", "audio/sfx/heal.wav"] {
            assert!(all.contains(&path), "{path}");
        }
        let mut sorted = all.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(all, sorted);
        let audio = files_under("audio");
        assert!(audio.contains(&"audio/audio.ron"));
        assert!(audio.contains(&"audio/sfx/heal.wav"));
        assert!(audio.iter().all(|p| p.starts_with("audio/")));
        assert_eq!(files_under("data"), files_in("data"));
        assert!(files_under("no_such_dir").is_empty());
    }

    #[test]
    fn display_path_prefixes_assets() {
        assert_eq!(display_path("data/palette.ron"), "assets/data/palette.ron");
    }

    /// Without the `private-assets` feature (every gate), `assets/` is the
    /// only layer: what a test reads never depends on the machine it runs on.
    #[cfg(not(feature = "private-assets"))]
    #[test]
    fn gates_read_only_the_public_assets() {
        assert_eq!(LAYERS.len(), 1);
        assert_eq!(LAYERS[0].display_root, DISPLAY_ROOT);
    }

    const fn entry(path: &'static str, contents: &'static str) -> DirEntry<'static> {
        DirEntry::File(File::new(path, contents.as_bytes()))
    }

    /// A stand-in for `assets-private/game/`.
    static PRIVATE_FIXTURE: Dir<'static> = Dir::new(
        "",
        &[
            DirEntry::Dir(Dir::new("data", &[entry("data/a.ron", "private a")])),
            DirEntry::Dir(Dir::new(
                "portraits",
                &[entry("portraits/new.portrait", "private only")],
            )),
            DirEntry::Dir(Dir::new(
                "deep",
                &[DirEntry::Dir(Dir::new(
                    "deep/x",
                    &[entry("deep/x/y.txt", "private y")],
                ))],
            )),
        ],
    );

    /// A stand-in for `assets/`.
    static PUBLIC_FIXTURE: Dir<'static> = Dir::new(
        "",
        &[
            entry("top.txt", "public top"),
            DirEntry::Dir(Dir::new(
                "data",
                &[
                    entry("data/a.ron", "public a"),
                    entry("data/b.ron", "public b"),
                ],
            )),
            DirEntry::Dir(Dir::new(
                "deep",
                &[DirEntry::Dir(Dir::new(
                    "deep/x",
                    &[entry("deep/x/z.txt", "public z")],
                ))],
            )),
        ],
    );

    /// The private stand-in laid over the public one, as a build with the
    /// `private-assets` feature has them.
    static MERGED: &[Layer] = &[
        Layer {
            display_root: PRIVATE_DISPLAY_ROOT,
            dir: &PRIVATE_FIXTURE,
        },
        Layer {
            display_root: DISPLAY_ROOT,
            dir: &PUBLIC_FIXTURE,
        },
    ];

    fn text(layers: &'static [Layer], path: &str) -> Option<&'static str> {
        find(layers, path)?.1.contents_utf8()
    }

    #[test]
    fn a_private_file_replaces_the_public_file_at_the_same_path() {
        assert_eq!(text(MERGED, "data/a.ron"), Some("private a"));
        assert_eq!(
            display_in(MERGED, "data/a.ron"),
            "assets-private/game/data/a.ron"
        );
        // Without the private layer the public file is back.
        assert_eq!(text(&MERGED[1..], "data/a.ron"), Some("public a"));
    }

    #[test]
    fn a_private_only_file_is_added() {
        assert_eq!(text(MERGED, "portraits/new.portrait"), Some("private only"));
        assert_eq!(list(MERGED, "portraits", false), ["portraits/new.portrait"]);
        assert_eq!(text(&MERGED[1..], "portraits/new.portrait"), None);
        assert!(list(&MERGED[1..], "portraits", false).is_empty());
    }

    #[test]
    fn a_public_only_file_is_still_read() {
        assert_eq!(text(MERGED, "data/b.ron"), Some("public b"));
        assert_eq!(display_in(MERGED, "data/b.ron"), "assets/data/b.ron");
    }

    #[test]
    fn a_file_in_no_layer_is_missing_and_shown_under_assets() {
        assert!(find(MERGED, "data/nope.ron").is_none());
        assert_eq!(display_in(MERGED, "data/nope.ron"), "assets/data/nope.ron");
    }

    #[test]
    fn lists_merge_the_layers_sorted_with_each_path_once() {
        assert_eq!(list(MERGED, "data", false), ["data/a.ron", "data/b.ron"]);
        assert_eq!(list(MERGED, "", false), ["top.txt"]);
        // `deep` has no files of its own, only in the directory below.
        assert!(list(MERGED, "deep", false).is_empty());
        assert_eq!(list(MERGED, "deep", true), ["deep/x/y.txt", "deep/x/z.txt"]);
        assert_eq!(
            list(MERGED, "", true),
            [
                "data/a.ron",
                "data/b.ron",
                "deep/x/y.txt",
                "deep/x/z.txt",
                "portraits/new.portrait",
                "top.txt",
            ]
        );
        assert!(list(MERGED, "no_such_dir", true).is_empty());
    }
}

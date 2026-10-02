//! The image table (ADR-0038): every PNG in the asset bundle except the font
//! atlas, by bundle path, with its size. Screens name a picture by an
//! [`ImageId`] from this table; `app` decodes and draws it. `content` reads
//! only each file's header, never its pixels.

use std::collections::BTreeMap;
use std::path::Path;

use crate::bundle;
use crate::error::ContentError;
use crate::font::ATLAS_PNG_PATH;

/// Path of the test card inside the bundle: a generated 16×16 image
/// (`cargo xtask test-card`) for tests and the "Sprite test" debug tool.
pub const TEST_CARD_PATH: &str = "images/test_card.png";

/// The longest side an image may have, in pixels: what every graphics card
/// the game runs on (WebGL included) takes as one texture.
pub const MAX_SIDE: u32 = 4096;

/// An image's size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageInfo {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// Names one image of an [`ImageTable`]: its interned bundle path. Cheap to
/// copy and compare. Only [`ImageTable::id`] makes one, so an id always
/// names an image some table holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageId(&'static str);

impl ImageId {
    /// The image's path in the bundle, e.g. `images/test_card.png`.
    pub const fn path(self) -> &'static str {
        self.0
    }
}

/// Every image the game can draw, by bundle path (relative to `assets/`,
/// `/`-separated).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageTable {
    /// Size by path.
    pub images: BTreeMap<&'static str, ImageInfo>,
}

impl ImageTable {
    /// Reads the size of every PNG in the embedded bundle (the font atlas
    /// aside, which [`crate::font`] checks), reporting every bad file.
    pub fn load() -> Result<Self, Vec<ContentError>> {
        let files = bundle::files_under("")
            .into_iter()
            .filter(|path| is_png(path) && *path != ATLAS_PNG_PATH)
            .map(|path| (path, bundle::bytes(path).unwrap_or_default()));
        Self::from_files(files)
    }

    /// The table of `files` (bundle path and contents), or every problem
    /// found: a file that isn't a PNG, an empty image, or one with a side
    /// over [`MAX_SIDE`].
    pub fn from_files<'a>(
        files: impl IntoIterator<Item = (&'static str, &'a [u8])>,
    ) -> Result<Self, Vec<ContentError>> {
        let mut images = BTreeMap::new();
        let mut errors = Vec::new();
        for (path, bytes) in files {
            match read_info(bytes) {
                Ok(info) => {
                    images.insert(path, info);
                }
                Err(message) => {
                    errors.push(ContentError::new(bundle::display_path(path), message));
                }
            }
        }
        if errors.is_empty() {
            Ok(Self { images })
        } else {
            Err(errors)
        }
    }

    /// The id of the image at `path`, or `None` if the table lacks it.
    pub fn id(&self, path: &str) -> Option<ImageId> {
        self.images
            .get_key_value(path)
            .map(|(&path, _)| ImageId(path))
    }

    /// The size of image `id`, or `None` if it is another table's.
    pub fn info(&self, id: ImageId) -> Option<ImageInfo> {
        self.images.get(id.path()).copied()
    }

    /// Every image's id, in path order.
    pub fn ids(&self) -> impl Iterator<Item = ImageId> + '_ {
        self.images.keys().map(|&path| ImageId(path))
    }
}

/// Whether `path` is named as a PNG (`.png`, any case).
fn is_png(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
}

/// The size in PNG file `bytes`, or what is wrong with it in plain words.
fn read_info(bytes: &[u8]) -> Result<ImageInfo, String> {
    let (width, height) = png_size(bytes).ok_or("not a valid PNG file")?;
    if width == 0 || height == 0 {
        return Err(format!(
            "image is {width}×{height} px; both sides must be greater than 0"
        ));
    }
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(format!(
            "image is {width}×{height} px; neither side may be over {MAX_SIDE}"
        ));
    }
    Ok(ImageInfo { width, height })
}

/// Width and height from a PNG's header, or `None` if `bytes` isn't a PNG.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    // Signature, then the IHDR chunk: length (4), type (4), width (4), height (4).
    let header = bytes.get(..24)?;
    if &header[..8] != SIGNATURE || &header[12..16] != b"IHDR" {
        return None;
    }
    let be = |at: usize| {
        u32::from_be_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
    };
    Some((be(16), be(20)))
}

/// The start of a `w × h` PNG file: enough for [`png_size`].
#[cfg(test)]
pub(crate) fn png_header(w: u32, h: u32) -> Vec<u8> {
    let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    v.extend(w.to_be_bytes());
    v.extend(h.to_be_bytes());
    v.extend([8, 6, 0, 0, 0]);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(result: Result<ImageTable, Vec<ContentError>>) -> Vec<String> {
        result
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn png_size_reads_header() {
        assert_eq!(png_size(&png_header(256, 288)), Some((256, 288)));
        // Distinct bytes, so any byte-order slip shows.
        assert_eq!(
            png_size(&png_header(0x0102_0304, 0x0506_0708)),
            Some((0x0102_0304, 0x0506_0708))
        );
        assert_eq!(png_size(&png_header(256, 288)[..23]), None);
        let mut bad = png_header(1, 1);
        bad[0] = 0;
        assert_eq!(png_size(&bad), None);
        let mut bad = png_header(1, 1);
        bad[12] = b'X';
        assert_eq!(png_size(&bad), None);
    }

    #[test]
    fn is_png_goes_by_the_extension() {
        assert!(is_png("images/a.png"));
        assert!(is_png("a.PNG"));
        assert!(!is_png("images/a.ron"));
        assert!(!is_png("images/png"));
        assert!(!is_png("a.png.txt"));
    }

    #[test]
    fn from_files_keeps_each_size_by_path() {
        // The biggest image allowed is `MAX_SIDE` each way.
        let (a, b) = (png_header(16, 24), png_header(MAX_SIDE, MAX_SIDE));
        let table = ImageTable::from_files([("x/b.png", &b[..]), ("a.png", &a[..])]);
        let table = table.unwrap_or_default();
        let sizes: Vec<_> = table.images.iter().collect();
        assert_eq!(
            sizes,
            [
                (
                    &"a.png",
                    &ImageInfo {
                        width: 16,
                        height: 24
                    }
                ),
                (
                    &"x/b.png",
                    &ImageInfo {
                        width: MAX_SIDE,
                        height: MAX_SIDE
                    }
                ),
            ]
        );
        assert_eq!(ImageTable::from_files([]), Ok(ImageTable::default()));
    }

    #[test]
    fn ids_name_images_by_path() {
        let png = png_header(16, 24);
        let table = ImageTable::from_files([("a.png", &png[..]), ("b.png", &png[..])]);
        let table = table.unwrap_or_default();
        let a = table.id("a.png");
        assert_eq!(a.map(ImageId::path), Some("a.png"));
        assert_ne!(a, table.id("b.png"));
        assert_eq!(table.id("c.png"), None);
        let size = ImageInfo {
            width: 16,
            height: 24,
        };
        assert_eq!(a.and_then(|id| table.info(id)), Some(size));
        let paths: Vec<_> = table.ids().map(ImageId::path).collect();
        assert_eq!(paths, ["a.png", "b.png"]);
        // An id of another table's image has no size here.
        assert_eq!(a.and_then(|id| ImageTable::default().info(id)), None);
    }

    #[test]
    fn every_bad_file_is_reported_by_name() {
        let ok = png_header(1, 1);
        let (wide, tall) = (png_header(MAX_SIDE + 1, 8), png_header(8, MAX_SIDE + 1));
        let (flat, thin) = (png_header(8, 0), png_header(0, 8));
        let files = [
            ("images/ok.png", &ok[..]),
            ("images/text.png", &b"not an image"[..]),
            ("images/wide.png", &wide[..]),
            ("images/tall.png", &tall[..]),
            ("images/flat.png", &flat[..]),
            ("images/thin.png", &thin[..]),
        ];
        assert_eq!(
            messages(ImageTable::from_files(files)),
            [
                "assets/images/text.png: not a valid PNG file",
                "assets/images/wide.png: image is 4097×8 px; neither side may be over 4096",
                "assets/images/tall.png: image is 8×4097 px; neither side may be over 4096",
                "assets/images/flat.png: image is 8×0 px; both sides must be greater than 0",
                "assets/images/thin.png: image is 0×8 px; both sides must be greater than 0",
            ]
        );
    }

    #[test]
    fn embedded_table_lists_the_test_card_and_not_the_font_atlas() {
        let table = ImageTable::load();
        assert!(table.is_ok(), "{table:?}");
        let table = table.unwrap_or_default();
        let card = table.id(TEST_CARD_PATH);
        assert_eq!(
            card.and_then(|id| table.info(id)),
            Some(ImageInfo {
                width: 16,
                height: 16
            })
        );
        assert_eq!(table.id(ATLAS_PNG_PATH), None);
        assert!(bundle::bytes(ATLAS_PNG_PATH).is_some());
    }
}

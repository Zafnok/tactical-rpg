use std::collections::BTreeMap;

use trpg_content::image::TEST_CARD_PATH;
use trpg_ui::{Cell, Overlay, Rect};

use super::*;

/// The console background in these tests.
const CLEAR: Rgb = Rgb::new(16, 16, 16);

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

/// A 24×16 atlas of three 8×16 cells: ' ', then 'A' (a solid 4×4 block
/// in its top-left corner) and the fallback '?' (its left column).
fn tiny_atlas() -> (FontAtlasDef, Vec<u8>) {
    let def = FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 3,
        glyphs: BTreeMap::from([(' ', 0), ('A', 1), (FALLBACK_GLYPH, 2)]),
    };
    let mut rgba = vec![0; 24 * 16 * 4];
    let mut set = |x: usize, y: usize| rgba[(y * 24 + x) * 4..][..4].copy_from_slice(&[255; 4]);
    for y in 0..4 {
        for x in 8..12 {
            set(x, y);
        }
    }
    for y in 0..16 {
        set(16, y);
    }
    (def, encode_png(24, 16, &rgba).unwrap())
}

fn test_card() -> &'static [u8] {
    trpg_content::bundle::bytes(TEST_CARD_PATH).unwrap()
}

/// A painter with the tiny atlas and the test card, if `with_card`.
fn painter(with_card: bool) -> (Painter, ImageTable) {
    let (atlas, png) = tiny_atlas();
    let table = ImageTable::from_files([(TEST_CARD_PATH, test_card())]).unwrap();
    let read = |path: &str| (with_card && path == TEST_CARD_PATH).then(test_card);
    (
        Painter::new(atlas, &png, &table, read, CLEAR).unwrap(),
        table,
    )
}

/// 3×2 cells: an 'A' on a blue cell, a glyph the atlas lacks, a green
/// `Under` line through the 'A', a red `Over` dot on it, a white `Over`
/// square, and the test card, flipped, half-opaque and clipped to its
/// middle-top 8×8, over the bottom row.
fn hand_checked_frame(table: &ImageTable) -> GlyphBuffer {
    let mut buf = GlyphBuffer::new(3, 2, Cell::new(' ', CLEAR, CLEAR));
    buf.set(
        0,
        0,
        Cell::new('A', Rgb::new(200, 100, 50), Rgb::new(0, 0, 128)),
    );
    buf.set(1, 0, Cell::new('Z', Rgb::new(1, 2, 3), CLEAR));
    let under = Overlay::new(Rect::new(0, 2, 8, 1), Rgb::new(0, 255, 0), Layer::Under);
    buf.add_overlay(under);
    buf.add_overlay(Overlay::new(
        Rect::new(3, 3, 1, 1),
        Rgb::new(255, 0, 0),
        Layer::Over,
    ));
    buf.add_overlay(Overlay::new(
        Rect::new(20, 0, 2, 2),
        Rgb::new(255, 255, 255),
        Layer::Over,
    ));
    let card = table.id(TEST_CARD_PATH).unwrap();
    let dest = Rect::new(0, 16, 16, 16);
    let mut sprite = Sprite::new(card, Rect::new(0, 0, 16, 16), dest, Layer::Over);
    sprite.clip = Rect::new(4, 16, 8, 8);
    sprite.flip_x = true;
    sprite.opacity = 128;
    buf.add_sprite(sprite);
    buf
}

/// Pixels of the hand-checked frame at scale 1, worked out by hand.
const EXPECTED: [((u32, u32), [u8; 3]); 15] = [
    // The 'A': its block in fg, the rest of the cell its bg.
    ((0, 0), [200, 100, 50]),
    ((2, 1), [200, 100, 50]),
    ((5, 5), [0, 0, 128]),
    // The `Under` line shows only beside the glyph.
    ((0, 2), [200, 100, 50]),
    ((5, 2), [0, 255, 0]),
    // The `Over` dot covers the glyph.
    ((3, 3), [255, 0, 0]),
    ((20, 1), [255, 255, 255]),
    // The missing 'Z' draws the fallback glyph in magenta.
    ((8, 7), [255, 0, 255]),
    ((9, 7), [16, 16, 16]),
    // The sprite, flipped: its left shows the card's right (green), its
    // right the card's left (red); 128/255 over the clear colour.
    ((4, 17), [34, 93, 42]),
    ((11, 17), [110, 34, 34]),
    ((6, 16), [136, 136, 136]),
    // Clipped: nothing beside or below the clip.
    ((3, 17), [16, 16, 16]),
    ((12, 17), [16, 16, 16]),
    ((6, 24), [16, 16, 16]),
];

#[test]
fn matches_a_hand_checked_frame() {
    let (painter, table) = painter(true);
    let buf = hand_checked_frame(&table);
    for scale in [1, 2] {
        let image = painter.render(&buf, scale);
        assert_eq!((image.width, image.height), (24 * scale, 32 * scale));
        for ((x, y), [r, g, b]) in EXPECTED {
            // Every pixel of the console pixel's square.
            for (dx, dy) in [(0, 0), (scale - 1, scale - 1)] {
                let at = (x * scale + dx, y * scale + dy);
                assert_eq!(
                    image.pixel(at.0, at.1),
                    Some([r, g, b, 255]),
                    "({x}, {y}) at scale {scale}"
                );
            }
        }
    }
}

#[test]
fn an_unflipped_solid_sprite_shows_the_card_as_is() {
    let (painter, table) = painter(true);
    let mut buf = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, CLEAR));
    let card = table.id(TEST_CARD_PATH).unwrap();
    let full = Rect::new(0, 0, 16, 16);
    buf.add_sprite(Sprite::new(card, full, full, Layer::Under));
    let image = painter.render(&buf, 1);
    for y in 0..16 {
        for x in 0..16 {
            let want = crate::test_card::pixels()[((y * 16 + x) * 4) as usize..][..4].to_vec();
            assert_eq!(image.pixel(x, y).unwrap().to_vec(), want, "({x}, {y})");
        }
    }
}

#[test]
fn a_missing_image_draws_magenta_where_the_sprite_shows() {
    let (painter, table) = painter(false);
    let image = painter.render(&hand_checked_frame(&table), 1);
    assert_eq!(image.pixel(4, 17), Some([255, 0, 255, 255]));
    assert_eq!(image.pixel(11, 23), Some([255, 0, 255, 255]));
    assert_eq!(image.pixel(12, 17), Some([16, 16, 16, 255]));
}

#[test]
fn a_bad_atlas_image_is_an_error() {
    let (atlas, _) = tiny_atlas();
    let table = ImageTable::default();
    let err = Painter::new(atlas, b"not a png", &table, |_| None, CLEAR).err();
    assert!(err.unwrap().starts_with("font atlas image"));
}

#[test]
fn parses_steps_in_order_and_options() {
    let parsed = parse_args(&args(&[
        "out.png", "--keys", "F2 Up", "--wait", "0.5", "--pad", "South", "--keys", "f", "--layout",
        "left", "--scale", "3", "--web",
    ]))
    .unwrap();
    assert_eq!(
        parsed,
        Options {
            out: PathBuf::from("out.png"),
            steps: vec![
                Step::Keys("F2 Up".into()),
                Step::Wait(0.5),
                Step::Pad("South".into()),
                Step::Keys("f".into()),
            ],
            layout: Layout::LeftHanded,
            scale: 3,
            web: true,
        }
    );
    let zero = parse_args(&args(&["a.png", "--wait", "0", "--scale", "8"])).unwrap();
    assert_eq!((zero.steps, zero.scale), (vec![Step::Wait(0.0)], 8));
    let plain = parse_args(&args(&["a.png"])).unwrap();
    assert_eq!(
        (plain.layout, plain.scale, plain.web),
        (Layout::RightHanded, 2, false)
    );
    assert!(plain.steps.is_empty());
}

#[test]
fn rejects_bad_arguments() {
    for bad in [
        &[][..],
        &["a.png", "b.png"],
        &["a.png", "--keys"],
        &["a.png", "--keys", "NoSuchKey"],
        &["a.png", "--pad", "Nope"],
        &["a.png", "--wait", "-1"],
        &["a.png", "--wait", "soon"],
        &["a.png", "--layout", "middle"],
        &["a.png", "--scale", "0"],
        &["a.png", "--scale", "9"],
        &["a.png", "--bogus"],
    ] {
        assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
    }
}

/// A fresh empty directory under the system temp directory.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-frame-png-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn writes_the_title_screen_and_the_same_bytes_twice() {
    let dir = temp_dir("title");
    let options = parse_args(&args(&["out/title.png", "--scale", "1"])).unwrap();
    let summary = run(&dir, &options).unwrap();
    assert!(summary.starts_with("frame-png: title → "), "{summary}");
    let first = fs::read(dir.join("out/title.png")).unwrap();
    let (w, h, rgba) = decode_png(&first).unwrap();
    assert_eq!((w, h), (800, 512));
    // The console background, opaque, in the corner.
    assert_eq!(&rgba[..4], &[0, 0, 0, 255]);
    // Something is drawn.
    assert!(rgba.chunks(4).any(|p| p[..3] != [0, 0, 0]));
    run(&dir, &options).unwrap();
    assert_eq!(fs::read(dir.join("out/title.png")).unwrap(), first);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn keys_reach_the_quick_battle_map() {
    let dir = temp_dir("battle");
    let options = parse_args(&args(&["b.png", "--keys", "Down f", "--scale", "1"])).unwrap();
    let summary = run(&dir, &options).unwrap();
    assert!(summary.starts_with("frame-png: battle → "), "{summary}");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_web_start_draws_another_title() {
    let dir = temp_dir("web");
    let render = |extra: &[&str]| {
        let mut all = vec!["t.png", "--scale", "1"];
        all.extend_from_slice(extra);
        run(&dir, &parse_args(&args(&all)).unwrap()).unwrap();
        fs::read(dir.join("t.png")).unwrap()
    };
    assert_ne!(render(&["--web"]), render(&[]));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_summary_warns_about_bought_art() {
    let frame = Image {
        width: 2,
        height: 1,
        rgba: vec![0; 8],
    };
    assert_eq!(
        summary("title → a.png", &frame, false),
        "frame-png: title → a.png (2×1)"
    );
    let warned = summary("title → a.png", &frame, true);
    assert!(warned.starts_with(
        "frame-png: title → a.png (2×1)
warning: "
    ));
    assert!(warned.contains("never commit it"), "{warned}");
}

#[test]
fn empty_pictures_and_pixels_off_the_picture_are_harmless() {
    for (width, height) in [(0, 0), (0, 3), (3, 0)] {
        let empty = Image {
            width,
            height,
            rgba: Vec::new(),
        };
        assert_eq!(empty.texel(1, 1), [0; 4]);
    }
    let mut image = Image {
        width: 2,
        height: 2,
        rgba: vec![7; 16],
    };
    for (x, y) in [(-1, 0), (0, -1), (2, 0), (0, 2)] {
        image.blend(x, y, [1.0; 3], 1.0);
    }
    assert_eq!(image.rgba, vec![7; 16]);
    // Texels clamp to the picture's edge.
    image.blend(1, 1, [1.0; 3], 1.0);
    assert_eq!(image.texel(5, 5), [255, 255, 255, 255]);
    assert_eq!(image.texel(-5, -5), [7; 4]);
    assert_eq!(image.pixel(2, 0), None);
}

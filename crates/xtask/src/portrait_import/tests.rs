use trpg_content::portrait::{REQUIRED_EXPRESSIONS, parse_portrait};
use trpg_content::{ImageInfo, ImageTable};

use super::*;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

/// A made-up `width × height` picture: pixel `(x, y)` is `[x, y, tag, 255]`,
/// so a cut shows where it was taken from.
fn made_up(width: u32, height: u32, tag: u8) -> Picture {
    let byte = |v: u32| u8::try_from(v % 256).unwrap();
    let pixels = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
    Picture {
        width,
        height,
        rgba: pixels
            .flat_map(|(x, y)| [byte(x), byte(y), tag, 255])
            .collect(),
    }
}

/// Pixel `(x, y)` of `picture`.
fn pixel(picture: &Picture, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * picture.width + x) * 4) as usize;
    [
        picture.rgba[at],
        picture.rgba[at + 1],
        picture.rgba[at + 2],
        picture.rgba[at + 3],
    ]
}

/// A fresh folder under the temp directory, with `assets-private/game/`.
fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xtask-portrait-import-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("assets-private/game")).unwrap();
    root
}

fn write_png(path: &Path, picture: &Picture) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let png = encode_png(picture.width, picture.height, &picture.rgba).unwrap();
    fs::write(path, png).unwrap();
}

fn options(source: &Path, character: &str, shift_x: i32) -> Options {
    Options {
        source: source.to_path_buf(),
        character: character.to_owned(),
        shift_x,
    }
}

#[test]
fn parse_args_reads_the_source_the_id_and_the_shift() {
    assert_eq!(
        parse_args(&args(&["busts", "knight_a"])),
        Ok(options(Path::new("busts"), "knight_a", 0))
    );
    for (text, shift) in [("3", 3), ("-8", -8), ("8", 8), ("0", 0)] {
        assert_eq!(
            parse_args(&args(&["sheet.png", "k2", "--shift-x", text])),
            Ok(options(Path::new("sheet.png"), "k2", shift))
        );
    }
}

#[test]
fn parse_args_rejects_what_it_cannot_use() {
    let err = |items: &[&str]| parse_args(&args(items)).unwrap_err();
    let usage = "expected <bust-folder-or-sheet> <character-id>";
    assert_eq!(err(&[]), usage);
    assert_eq!(err(&["busts"]), usage);
    assert_eq!(err(&["busts", "k", "--shift-x"]), usage);
    assert_eq!(err(&["busts", "k", "--bogus", "1"]), usage);
    assert_eq!(err(&["busts", "k", "--shift-x", "1", "2"]), usage);
    for id in ["", "Knight", "a b", "a/b", "a.b", "é"] {
        assert_eq!(
            err(&["busts", id]),
            format!("character id \"{id}\" must be lower-case letters, digits and _")
        );
    }
    for shift in ["9", "-9", "1.5", "x", ""] {
        assert_eq!(
            err(&["busts", "k", "--shift-x", shift]),
            format!("--shift-x must be a whole number from -8 to 8, not \"{shift}\"")
        );
    }
}

#[test]
fn crop_takes_the_named_part() {
    let part = made_up(10, 6, 0).crop(3, 2, 4, 3);
    assert_eq!((part.width, part.height), (4, 3));
    assert_eq!(part.rgba.len(), 4 * 3 * 4);
    assert_eq!(pixel(&part, 0, 0), [3, 2, 0, 255]);
    assert_eq!(pixel(&part, 3, 0), [6, 2, 0, 255]);
    assert_eq!(pixel(&part, 0, 2), [3, 4, 0, 255]);
    assert_eq!(pixel(&part, 3, 2), [6, 4, 0, 255]);
    // The whole picture is itself.
    let whole = made_up(10, 6, 0);
    assert_eq!(whole.crop(0, 0, 10, 6), whole);
}

#[test]
fn a_bust_is_cut_to_its_middle_64_columns_and_bottom_64_rows() {
    let cut = portrait_image("b", &made_up(80, 80, 7), 0).unwrap();
    assert_eq!((cut.width, cut.height), (64, 64));
    // Columns 8..72, rows 16..80.
    assert_eq!(pixel(&cut, 0, 0), [8, 16, 7, 255]);
    assert_eq!(pixel(&cut, 63, 0), [71, 16, 7, 255]);
    assert_eq!(pixel(&cut, 0, 63), [8, 79, 7, 255]);
    assert_eq!(pixel(&cut, 63, 63), [71, 79, 7, 255]);
}

#[test]
fn shift_x_moves_the_cut_sideways_only() {
    for (shift, left) in [(3, 11), (-3, 5), (8, 16), (-8, 0), (1, 9)] {
        let cut = portrait_image("b", &made_up(80, 80, 0), shift).unwrap();
        assert_eq!((cut.width, cut.height), (64, 64), "{shift}");
        assert_eq!(pixel(&cut, 0, 0), [left, 16, 0, 255], "{shift}");
        assert_eq!(pixel(&cut, 63, 63), [left + 63, 79, 0, 255], "{shift}");
    }
    // Past the range it stops at the bust's edge instead of reading outside.
    assert_eq!(cut_left(100), 16);
    assert_eq!(cut_left(-100), 0);
}

#[test]
fn a_face_is_kept_whole_whatever_the_shift() {
    let face = made_up(48, 48, 1);
    assert_eq!(portrait_image("f", &face, 0), Ok(face.clone()));
    assert_eq!(portrait_image("f", &face, 5), Ok(face));
}

#[test]
fn any_other_size_is_an_error_that_names_the_file() {
    for (w, h) in [(64, 64), (80, 48), (48, 80), (144, 144), (81, 80)] {
        assert_eq!(
            portrait_image("x/bust_odd.png", &made_up(w, h, 0), 0),
            Err(format!(
                "x/bust_odd.png is {w}×{h} px; a bust is 80×80 and a face 48×48"
            ))
        );
    }
}

/// A sheet of 4×2 cells of `side`: cell `n`'s pixels are `[x, y, n, 255]`
/// with `(x, y)` counted inside the cell.
fn sheet(side: u32) -> Picture {
    let mut rgba = Vec::new();
    for y in 0..side * 2 {
        for x in 0..side * 4 {
            let n = u8::try_from((y / side) * 4 + x / side).unwrap();
            let byte = |v: u32| u8::try_from(v % side).unwrap();
            rgba.extend([byte(x), byte(y), n, 255]);
        }
    }
    Picture {
        width: side * 4,
        height: side * 2,
        rgba,
    }
}

#[test]
fn a_sheet_is_split_into_its_eight_expressions_row_by_row() {
    for side in [80, 48] {
        let cells = split_sheet("s.png", &sheet(side)).unwrap();
        let names: Vec<&str> = cells.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, SHEET_EXPRESSIONS);
        for (n, (_, cell)) in cells.iter().enumerate() {
            let n = u8::try_from(n).unwrap();
            let last = u8::try_from(side - 1).unwrap();
            assert_eq!((cell.width, cell.height), (side, side));
            assert_eq!(pixel(cell, 0, 0), [0, 0, n, 255]);
            assert_eq!(pixel(cell, side - 1, side - 1), [last, last, n, 255]);
        }
    }
    assert_eq!(
        split_sheet("s.png", &made_up(576, 288, 0)),
        Err("s.png is 576×288 px; a bust sheet is 320×160 and a face sheet 192×96".to_owned())
    );
    assert!(split_sheet("s.png", &made_up(320, 96, 0)).is_err());
    assert!(split_sheet("s.png", &made_up(192, 160, 0)).is_err());
}

#[test]
fn a_folder_gives_its_bust_files_before_its_faces_before_everything() {
    let stems = |names: &[&str]| -> Vec<String> { args(names) };
    let all = stems(&[
        "battler",
        "bust_neutral",
        "bust_sad",
        "bust_sheet",
        "face_neutral",
        "faceset_sheet",
    ]);
    assert_eq!(pick_files(&all), ["bust_neutral", "bust_sad"]);
    let faces = stems(&["battler", "face_neutral", "face_sheet", "faceset_sheet"]);
    assert_eq!(pick_files(&faces), ["face_neutral"]);
    let other = stems(&["knight_neutral", "knight_sad"]);
    assert_eq!(pick_files(&other), ["knight_neutral", "knight_sad"]);
    assert!(pick_files(&[]).is_empty());
    assert_eq!(expression_of("bust_neutral"), "neutral");
    assert_eq!(expression_of("bust_dark_knight_sly"), "sly");
    assert_eq!(expression_of("neutral"), "neutral");
}

#[test]
fn the_stub_is_a_sidecar_the_game_loads() {
    let bought: Vec<String> = SHEET_EXPRESSIONS.iter().map(|e| (*e).to_owned()).collect();
    let stub = sidecar_stub("knight_a", &bought);
    assert_eq!(
        stub,
        "// Made by `cargo xtask portrait-import`. The first five are the expressions\n\
         // dialogue needs: point each at the closest bought one (ticket 0706).\n\
         (\n    character: \"knight_a\",\n    expressions: {\n        \
         \"neutral\": \"knight_a/neutral.png\",\n        \
         \"happy\": \"knight_a/smile.png\",\n        \
         \"angry\": \"knight_a/stern.png\",\n        \
         \"sad\": \"knight_a/sad.png\",\n        \
         \"surprised\": \"knight_a/surprise.png\",\n        \
         \"smile\": \"knight_a/smile.png\",\n        \
         \"sly\": \"knight_a/sly.png\",\n        \
         \"thinking\": \"knight_a/thinking.png\",\n        \
         \"stern\": \"knight_a/stern.png\",\n        \
         \"surprise\": \"knight_a/surprise.png\",\n        \
         \"unique\": \"knight_a/unique.png\",\n    },\n)\n"
    );
    let size = ImageInfo {
        width: CUT,
        height: CUT,
    };
    let images = ImageTable {
        images: bought
            .iter()
            .map(|e| -> &'static str { format!("portraits/knight_a/{e}.png").leak() })
            .map(|path| (path, size))
            .collect(),
    };
    let portrait = parse_portrait("knight_a.ron", "knight_a", &stub, &images).unwrap();
    assert_eq!(portrait.expressions.len(), 11);
    for required in REQUIRED_EXPRESSIONS {
        assert!(portrait.expression(required).is_some(), "{required}");
    }
    let image = |name| portrait.expression(name).map(|e| e.image.path());
    assert_eq!(image("angry"), Some("portraits/knight_a/stern.png"));
}

#[test]
fn the_stub_copes_with_other_spellings_and_missing_expressions() {
    // Heroes 2 spells one "surprised"; with no close one, the first is used.
    let stub = sidecar_stub("k", &args(&["neutral", "surprised"]));
    assert!(
        stub.contains("\"surprised\": \"k/surprised.png\""),
        "{stub}"
    );
    assert!(stub.contains("\"happy\": \"k/neutral.png\""), "{stub}");
    assert_eq!(stub.matches("\"surprised\":").count(), 1, "{stub}");
    assert_eq!(stub.matches("\"neutral\":").count(), 1, "{stub}");
    // Nothing bought: no entries (the loader then says what is missing).
    assert!(!sidecar_stub("k", &[]).contains(".png"));
}

#[test]
fn run_cuts_a_folder_of_busts_and_writes_the_sidecar_once() {
    let root = temp_root("folder");
    let source = root.join("library/Knight");
    for (n, expression) in ["Neutral", "smile", "stern", "sad", "surprise"]
        .iter()
        .enumerate()
    {
        let tag = u8::try_from(n).unwrap();
        let file = source.join(format!("bust_{expression}.png"));
        write_png(&file, &made_up(80, 80, tag));
    }
    // Other files of a hero's folder are left alone, whatever their size.
    write_png(&source.join("bust_sheet.png"), &made_up(320, 160, 0));
    write_png(&source.join("face_neutral.png"), &made_up(48, 48, 0));
    write_png(&source.join("map_sprite.png"), &made_up(12, 20, 0));
    fs::write(source.join("notes.txt"), "not an image").unwrap();

    let summary = run(&root, &options(&source, "knight_a", 2)).unwrap();
    assert_eq!(
        summary,
        "portrait-import: wrote 5 images to assets-private/game/portraits/knight_a/ \
         (neutral, sad, smile, stern, surprise); wrote a sidecar to fill in, \
         assets-private/game/portraits/knight_a.ron"
    );
    let out = root.join(OUT_DIR);
    let mut written: Vec<String> = fs::read_dir(out.join("knight_a"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    written.sort();
    assert_eq!(
        written,
        [
            "neutral.png",
            "sad.png",
            "smile.png",
            "stern.png",
            "surprise.png"
        ]
    );
    let stern = read_picture(&out.join("knight_a/stern.png")).unwrap();
    assert_eq!((stern.width, stern.height), (64, 64));
    assert_eq!(pixel(&stern, 0, 0), [10, 16, 2, 255]);
    let sidecar = fs::read_to_string(out.join("knight_a.ron")).unwrap();
    assert_eq!(
        sidecar,
        sidecar_stub(
            "knight_a",
            &args(&["neutral", "sad", "smile", "stern", "surprise"])
        )
    );

    // A second run cuts again (here unshifted) and keeps the sidecar.
    fs::write(out.join("knight_a.ron"), "filled in").unwrap();
    let summary = run(&root, &options(&source, "knight_a", 0)).unwrap();
    assert!(
        summary.ends_with(
            "; kept the sidecar that was there, assets-private/game/portraits/knight_a.ron"
        ),
        "{summary}"
    );
    assert_eq!(
        fs::read_to_string(out.join("knight_a.ron")).unwrap(),
        "filled in"
    );
    let stern = read_picture(&out.join("knight_a/stern.png")).unwrap();
    assert_eq!(pixel(&stern, 0, 0), [8, 16, 2, 255]);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn run_cuts_a_bust_sheet_and_keeps_faces_whole() {
    let root = temp_root("sheet");
    let bust_sheet = root.join("bust_sheet.png");
    write_png(&bust_sheet, &sheet(80));
    let summary = run(&root, &options(&bust_sheet, "b", 0)).unwrap();
    assert!(
        summary.starts_with("portrait-import: wrote 8 images"),
        "{summary}"
    );
    let out = root.join(OUT_DIR);
    for (n, expression) in SHEET_EXPRESSIONS.iter().enumerate() {
        let cut = read_picture(&out.join(format!("b/{expression}.png"))).unwrap();
        assert_eq!((cut.width, cut.height), (64, 64));
        assert_eq!(pixel(&cut, 0, 0), [8, 16, u8::try_from(n).unwrap(), 255]);
    }
    let face_sheet = root.join("faceset_sheet.png");
    write_png(&face_sheet, &sheet(48));
    run(&root, &options(&face_sheet, "f", 4)).unwrap();
    let face = read_picture(&out.join("f/sly.png")).unwrap();
    assert_eq!((face.width, face.height), (48, 48));
    assert_eq!(pixel(&face, 0, 0), [0, 0, 3, 255]);
    assert_eq!(pixel(&face, 47, 47), [47, 47, 3, 255]);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn run_writes_nothing_when_a_file_is_wrong() {
    let root = temp_root("wrong");
    let out = root.join(OUT_DIR);
    let source = root.join("busts");
    write_png(&source.join("bust_neutral.png"), &made_up(80, 80, 0));
    write_png(&source.join("bust_sad.png"), &made_up(144, 144, 0));
    let err = run(&root, &options(&source, "k", 0)).unwrap_err();
    assert!(
        err.ends_with("(sad) is 144×144 px; a bust is 80×80 and a face 48×48"),
        "{err}"
    );
    assert!(err.contains("busts"), "{err}");
    assert!(!out.exists());

    // Two files for one expression.
    let twice = root.join("twice");
    write_png(&twice.join("a_neutral.png"), &made_up(80, 80, 0));
    write_png(&twice.join("b_neutral.png"), &made_up(80, 80, 0));
    let err = run(&root, &options(&twice, "k", 0)).unwrap_err();
    assert!(
        err.ends_with("b_neutral.png is a second file for expression \"neutral\""),
        "{err}"
    );

    // No PNGs, a file that isn't a PNG, a source that isn't there.
    let empty = root.join("empty");
    fs::create_dir_all(&empty).unwrap();
    fs::write(empty.join("notes.txt"), "").unwrap();
    let err = run(&root, &options(&empty, "k", 0)).unwrap_err();
    assert!(err.ends_with("has no PNG files"), "{err}");
    let bad = root.join("bad");
    fs::create_dir_all(&bad).unwrap();
    fs::write(bad.join("bust_neutral.png"), "not an image").unwrap();
    let err = run(&root, &options(&bad, "k", 0)).unwrap_err();
    assert!(err.contains("bust_neutral.png"), "{err}");
    let err = run(&root, &options(&root.join("nowhere"), "k", 0)).unwrap_err();
    assert!(
        err.starts_with("reading ") && err.contains("nowhere"),
        "{err}"
    );
    assert!(!out.exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn run_needs_the_private_assets_checkout() {
    let root =
        std::env::temp_dir().join(format!("xtask-portrait-import-none-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    assert_eq!(
        run(&root, &options(Path::new("busts"), "k", 0)),
        Err(
            "assets-private/game/ is missing: run `cargo xtask private-assets --library` first"
                .to_owned()
        )
    );
    fs::remove_dir_all(&root).unwrap();
}

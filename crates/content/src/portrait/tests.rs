use super::*;
use crate::image::png_header;

/// An image table of `files`: bundle path, width, height.
fn table(files: &[(&'static str, u32, u32)]) -> ImageTable {
    let pngs: Vec<_> = files
        .iter()
        .map(|&(path, w, h)| (path, png_header(w, h)))
        .collect();
    ImageTable::from_files(pngs.iter().map(|(path, png)| (*path, &png[..]))).unwrap_or_default()
}

/// One 64×64 image per required expression, `portraits/t/<name>.png`, plus
/// a 48×48 `sly`, an image as big as the frame and three that are too big.
fn images() -> ImageTable {
    table(&[
        ("portraits/t/neutral.png", 64, 64),
        ("portraits/t/happy.png", 64, 64),
        ("portraits/t/angry.png", 64, 64),
        ("portraits/t/sad.png", 64, 64),
        ("portraits/t/surprised.png", 64, 64),
        ("portraits/t/sly.png", 48, 48),
        ("portraits/t/full.png", 256, 256),
        ("portraits/t/wide.png", 257, 64),
        ("portraits/t/tall.png", 64, 257),
        ("portraits/t/huge.png", 512, 512),
        // Not under `portraits/`: a sidecar can't name it.
        ("images/neutral.png", 64, 64),
    ])
}

/// A sidecar for character `t` with `entries` (name, file) as its
/// expressions, one per line from line 4 on.
fn sidecar(entries: &[(&str, &str)]) -> String {
    let lines: Vec<String> = entries
        .iter()
        .map(|(name, file)| format!("        \"{name}\": \"{file}\",\n"))
        .collect();
    let lines = lines.concat();
    format!("(\n    character: \"t\",\n    expressions: {{\n{lines}    }},\n)\n")
}

/// The required expressions, each mapped to its own image.
const REQUIRED: [(&str, &str); 5] = [
    ("neutral", "t/neutral.png"),
    ("happy", "t/happy.png"),
    ("angry", "t/angry.png"),
    ("sad", "t/sad.png"),
    ("surprised", "t/surprised.png"),
];

/// [`REQUIRED`] and `more`.
fn with(more: &[(&'static str, &'static str)]) -> String {
    let mut entries = REQUIRED.to_vec();
    entries.extend_from_slice(more);
    sidecar(&entries)
}

fn parse(src: &str) -> Result<Portrait, Vec<ContentError>> {
    parse_portrait("t.ron", "t", src, &images())
}

fn errors(src: &str) -> Vec<String> {
    parse(src)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn parses_a_valid_portrait() {
    let p = parse(&sidecar(&REQUIRED)).unwrap();
    assert_eq!(p.character, "t");
    let names: Vec<&str> = p.expressions.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, REQUIRED_EXPRESSIONS);
    let happy = p.expression("happy").unwrap();
    assert_eq!(happy.image.path(), "portraits/t/happy.png");
    assert_eq!(
        happy.size,
        ImageInfo {
            width: 64,
            height: 64
        }
    );
    assert_eq!(p.expression("bored"), None);
}

#[test]
fn an_expression_may_use_any_image_and_two_may_share_one() {
    let src = sidecar(&[
        ("neutral", "t/neutral.png"),
        ("happy", "t/sly.png"),
        ("angry", "t/neutral.png"),
        ("sad", "t/sad.png"),
        ("surprised", "t/surprised.png"),
    ]);
    let p = parse(&src).unwrap();
    let image = |name| p.expression(name).map(|e| e.image.path());
    assert_eq!(image("happy"), Some("portraits/t/sly.png"));
    assert_eq!(image("angry"), image("neutral"));
    assert_eq!(
        p.expression("happy").map(|e| e.size),
        Some(ImageInfo {
            width: 48,
            height: 48
        })
    );
}

#[test]
fn required_expressions_come_first_then_the_others_by_name() {
    // Written in another order, with extras before and between.
    let src = sidecar(&[
        ("wry", "t/sly.png"),
        ("surprised", "t/surprised.png"),
        ("sad", "t/sad.png"),
        ("bored", "t/sly.png"),
        ("angry", "t/angry.png"),
        ("happy", "t/happy.png"),
        ("neutral", "t/neutral.png"),
    ]);
    let p = parse(&src).unwrap();
    let names: Vec<&str> = p.expressions.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "neutral",
            "happy",
            "angry",
            "sad",
            "surprised",
            "bored",
            "wry"
        ]
    );
}

#[test]
fn an_image_as_big_as_the_frame_fits() {
    let p = parse(&with(&[("big", "t/full.png")])).unwrap();
    let (width, height) = FRAME_PX;
    assert_eq!(
        p.expression("big").map(|e| e.size),
        Some(ImageInfo { width, height })
    );
}

#[test]
fn syntax_error() {
    let errs = errors("(character: \"t\", expressions: {");
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].starts_with("t.ron:1:"), "{errs:?}");
    // An unknown field is one too.
    let errs = errors("(character: \"t\", size: (32, 32), expressions: {})");
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].contains("size"), "{errs:?}");
}

#[test]
fn stem_must_match_character() {
    let errs = parse_portrait("ana.ron", "ana", &sidecar(&REQUIRED), &images());
    let errs: Vec<String> = errs.unwrap_err().iter().map(ToString::to_string).collect();
    assert_eq!(
        errs,
        ["ana.ron: character \"t\" doesn't match the file name; name the file t.ron"]
    );
}

#[test]
fn missing_required_expressions() {
    let src = sidecar(&[("neutral", "t/neutral.png"), ("angry", "t/angry.png")]);
    assert_eq!(
        errors(&src),
        [
            "t.ron: missing required expression \"happy\"",
            "t.ron: missing required expression \"sad\"",
            "t.ron: missing required expression \"surprised\"",
        ]
    );
}

#[test]
fn an_image_must_be_a_png_under_the_portraits_directory() {
    // No such file; a file that isn't an image; an image elsewhere.
    let src = with(&[
        ("a", "t/nope.png"),
        ("b", "t.ron"),
        ("c", "../images/neutral.png"),
        ("d", "neutral.png"),
    ]);
    assert_eq!(
        errors(&src),
        [
            "t.ron:9: expression \"a\": assets/portraits/t/nope.png is not a PNG image in the assets",
            "t.ron:10: expression \"b\": assets/portraits/t.ron is not a PNG image in the assets",
            "t.ron:11: expression \"c\": assets/portraits/../images/neutral.png is not a PNG image in the assets",
            "t.ron:12: expression \"d\": assets/portraits/neutral.png is not a PNG image in the assets",
        ]
    );
}

#[test]
fn an_image_must_fit_the_frame() {
    let src = with(&[
        ("a", "t/wide.png"),
        ("b", "t/tall.png"),
        ("c", "t/huge.png"),
    ]);
    assert_eq!(
        errors(&src),
        [
            "t.ron:9: expression \"a\": assets/portraits/t/wide.png is 257×64 px; a portrait must fit 256×256 px",
            "t.ron:10: expression \"b\": assets/portraits/t/tall.png is 64×257 px; a portrait must fit 256×256 px",
            "t.ron:11: expression \"c\": assets/portraits/t/huge.png is 512×512 px; a portrait must fit 256×256 px",
        ]
    );
}

/// A file named `.png` that can't be read is reported by the image table,
/// by name, and a sidecar naming it is told it is no image.
#[test]
fn an_unreadable_png_is_reported_by_the_image_table() {
    let unreadable = ImageTable::from_files([("portraits/t/neutral.png", &b"not an image"[..])]);
    let errs: Vec<String> = unreadable
        .unwrap_err()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errs,
        ["assets/portraits/t/neutral.png: not a valid PNG file"]
    );
    let errs = parse_portrait(
        "t.ron",
        "t",
        &sidecar(&REQUIRED[..1]),
        &ImageTable::default(),
    );
    assert_eq!(
        errs.unwrap_err()[0].to_string(),
        "t.ron:4: expression \"neutral\": assets/portraits/t/neutral.png is not a PNG image in the assets"
    );
}

#[test]
fn reports_every_problem_at_once() {
    let src = "(\n    character: \"u\",\n    expressions: {\n        \"neutral\": \"t/wide.png\",\n        \"happy\": \"t/nope.png\",\n    },\n)\n";
    assert_eq!(
        errors(src),
        [
            "t.ron: character \"u\" doesn't match the file name; name the file u.ron",
            "t.ron:5: expression \"happy\": assets/portraits/t/nope.png is not a PNG image in the assets",
            "t.ron:4: expression \"neutral\": assets/portraits/t/wide.png is 257×64 px; a portrait must fit 256×256 px",
            "t.ron: missing required expression \"angry\"",
            "t.ron: missing required expression \"sad\"",
            "t.ron: missing required expression \"surprised\"",
        ]
    );
}

#[test]
fn embedded_placeholders_load() {
    let images = ImageTable::load().unwrap();
    let portraits = load_all(&images).unwrap();
    for id in ["lead_f", "lead_m", "test_knight", "test_lord"] {
        let p = &portraits[id];
        assert_eq!(p.character, id);
        let names: Vec<&str> = p.expressions.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, REQUIRED_EXPRESSIONS, "{id}");
        for e in &p.expressions {
            assert_eq!(e.image.path(), format!("portraits/{id}/{}.png", e.name));
            assert_eq!((e.size.width, e.size.height), (32, 32));
        }
    }
    // Only sidecars are portraits: not the README, not the images.
    assert_eq!(portraits.len(), 4);
}

#[test]
fn load_all_reports_every_file() {
    // Without the images, every expression of every sidecar is wrong.
    let errs = load_all(&ImageTable::default()).unwrap_err();
    assert_eq!(errs.len(), 4 * REQUIRED_EXPRESSIONS.len());
    assert_eq!(
        errs[0].to_string(),
        "assets/portraits/lead_f.ron:9: expression \"angry\": assets/portraits/lead_f/angry.png is not a PNG image in the assets"
    );
    assert!(
        errs.iter()
            .any(|e| e.file == "assets/portraits/test_lord.ron")
    );
}

use super::*;

const HEADER: &str = "(
    character: \"t\",
    size: (32, 32),
    colors: { 's': \"skin\", 'h': \"hair\" },
)
";

/// First row of every test expression: 16 hair pixels then 16 transparent.
fn top_row() -> String {
    format!("{}{}", "h".repeat(16), ".".repeat(16))
}

/// An expression block: the top row, then 31 rows of skin.
fn block(name: &str) -> String {
    let mut s = format!("=== {name}\n{}\n", top_row());
    for _ in 1..32 {
        s.push_str(&"s".repeat(32));
        s.push('\n');
    }
    s
}

/// A valid portrait with the required expressions. The header is lines 1–5,
/// `=== neutral` line 6, its rows lines 7–38, `=== happy` line 39, …
fn valid() -> String {
    let mut s = HEADER.to_owned();
    for name in REQUIRED_EXPRESSIONS {
        s.push_str(&block(name));
    }
    s
}

fn palette() -> PaletteDef {
    PaletteDef {
        colors: [("skin", [1, 2, 3]), ("hair", [4, 5, 6])]
            .into_iter()
            .map(|(n, c)| (n.to_owned(), c))
            .collect(),
    }
}

fn parse(src: &str) -> Result<Portrait, Vec<ContentError>> {
    parse_portrait("t.portrait", src, &palette())
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
    let p = parse(&valid()).unwrap();
    assert_eq!(p.character, "t");
    assert_eq!((p.width, p.height), (32, 32));
    assert_eq!(p.colors.get(&'s').map(String::as_str), Some("skin"));
    let names: Vec<&str> = p.expressions.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, REQUIRED_EXPRESSIONS);
    let happy = p.expression("happy").unwrap();
    assert_eq!(happy.pixels.len(), 32 * 32);
    assert_eq!(p.color_at(happy, 0, 0), Some("hair"));
    assert_eq!(p.color_at(happy, 15, 0), Some("hair"));
    assert_eq!(p.color_at(happy, 16, 0), None);
    assert_eq!(p.color_at(happy, 16, 1), Some("skin"));
    assert_eq!(p.color_at(happy, 31, 31), Some("skin"));
    assert_eq!(p.color_at(happy, 32, 0), None);
    assert_eq!(p.color_at(happy, 0, 32), None);
    assert!(p.expression("bored").is_none());
}

#[test]
fn color_at_ignores_keys_without_a_colour() {
    let p = Portrait {
        character: "t".to_owned(),
        width: 2,
        height: 1,
        colors: BTreeMap::new(),
        expressions: vec![],
    };
    let e = Expression {
        name: "x".to_owned(),
        pixels: vec![Some('z')],
    };
    assert_eq!(p.color_at(&e, 0, 0), None);
    // Too few pixels for the size.
    assert_eq!(p.color_at(&e, 1, 0), None);
}

#[test]
fn crlf_extra_expressions_and_trailing_blank_lines_are_fine() {
    let src = format!("{}{}\n\n", valid(), block("smug")).replace('\n', "\r\n");
    let p = parse(&src).unwrap();
    assert_eq!(p.expressions.len(), 6);
    assert_eq!(p.expressions[5].name, "smug");
    assert_eq!(
        p.expressions[5],
        Expression {
            name: "smug".to_owned(),
            ..p.expressions[0].clone()
        }
    );
}

#[test]
fn no_expressions() {
    assert_eq!(
        errors(HEADER),
        ["t.portrait: no expressions: each starts with a \"=== <name>\" line"]
    );
}

#[test]
fn header_syntax_error() {
    let src = valid().replacen("size", "size size", 1);
    let errs = errors(&src);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].starts_with("t.portrait:3:"), "{errs:?}");
}

#[test]
fn size_must_be_32_by_32() {
    let src = valid().replacen("(32, 32)", "(32, 31)", 1);
    let errs = errors(&src);
    assert_eq!(
        errs[0],
        "t.portrait:3: size is (32, 31); portraits are (32, 32) pixels"
    );
    // The grids are checked against the declared size too.
    assert_eq!(
        errs[1],
        "t.portrait:6:1: expression has 32 rows; expected 31"
    );
    assert_eq!(errs.len(), 6);
    let src = valid().replacen("(32, 32)", "(31, 32)", 1);
    assert_eq!(
        errors(&src)[0],
        "t.portrait:3: size is (31, 32); portraits are (32, 32) pixels"
    );
}

#[test]
fn size_error_names_the_size_line() {
    let src = valid().replacen("size: (32, 32)", "size:\n(1, 1)", 1);
    assert!(errors(&src)[0].starts_with("t.portrait:3: size is (1, 1)"));
    let src = valid().replacen("    size: (32, 32),\n", "", 1).replacen(
        "character: \"t\",",
        "character: \"t\", size: (2, 2),",
        1,
    );
    assert!(errors(&src)[0].starts_with("t.portrait:2: size is (2, 2)"));
}

#[test]
fn transparent_key_is_reserved() {
    let src = valid().replacen("'h': \"hair\"", "'h': \"hair\", '.': \"skin\"", 1);
    assert_eq!(
        errors(&src),
        ["t.portrait:4:41: colour '.': '.' is reserved for transparent pixels"]
    );
}

#[test]
fn unknown_palette_colour() {
    let src = valid().replacen("\"hair\"", "\"hare\"", 1);
    assert_eq!(
        errors(&src),
        ["t.portrait:4:28: colour 'h': unknown palette colour \"hare\""]
    );
}

#[test]
fn unknown_colour_key() {
    let src = valid().replacen(&top_row(), &format!("hhq{}", &top_row()[3..]), 1);
    assert_eq!(errors(&src), ["t.portrait:7:3: 'q' is not in the colours"]);
}

#[test]
fn ragged_rows() {
    let row = top_row();
    let src = valid().replacen(&row, &format!("{row}s"), 1);
    assert_eq!(
        errors(&src),
        ["t.portrait:7:33: row is 33 pixels wide; expected 32"]
    );
    let src = valid().replacen(&row, &row[1..], 1);
    assert_eq!(
        errors(&src),
        ["t.portrait:7:32: row is 31 pixels wide; expected 32"]
    );
}

#[test]
fn wrong_row_count() {
    let row = format!("{}\n", top_row());
    let src = valid().replacen(&row, "", 1);
    assert_eq!(
        errors(&src),
        ["t.portrait:6:1: expression has 31 rows; expected 32"]
    );
    let src = valid().replacen(&row, &format!("{row}{row}"), 1);
    assert_eq!(
        errors(&src),
        ["t.portrait:6:1: expression has 33 rows; expected 32"]
    );
}

#[test]
fn blank_lines_only_are_no_rows() {
    let src = format!(
        "{}=== smug


",
        valid()
    );
    assert_eq!(
        errors(&src),
        ["t.portrait:171:1: expression has 0 rows; expected 32"]
    );
}

#[test]
fn duplicate_and_unnamed_expressions() {
    let src = format!("{}{}", valid(), block("happy"));
    assert_eq!(
        errors(&src),
        ["t.portrait:171:1: expression \"happy\" appears twice (first on line 39)"]
    );
    let src = format!("{}{}", valid(), block("").replacen("=== ", "===", 1));
    assert_eq!(errors(&src), ["t.portrait:171:1: expression has no name"]);
}

#[test]
fn missing_required_expressions() {
    let src = format!("{HEADER}{}{}", block("neutral"), block("happy"));
    assert_eq!(
        errors(&src),
        [
            "t.portrait: missing required expression \"angry\"",
            "t.portrait: missing required expression \"sad\"",
            "t.portrait: missing required expression \"surprised\"",
        ]
    );
}

#[test]
fn reports_every_problem_at_once() {
    let src = valid()
        .replacen("\"hair\"", "\"hare\"", 1)
        .replacen(&top_row(), "q", 1);
    assert_eq!(errors(&src).len(), 3);
}

#[test]
fn stem_must_match_character() {
    let p = parse(&valid()).unwrap();
    assert_eq!(check_stem("f", "t", p.clone()), Ok(p.clone()));
    let errs = check_stem("assets/portraits/u.portrait", "u", p).unwrap_err();
    assert_eq!(
        errs[0].to_string(),
        "assets/portraits/u.portrait: character \"t\" doesn't match the file name; name the file t.portrait"
    );
}

#[test]
fn embedded_placeholders_load() {
    let palette = PaletteDef::load().unwrap();
    let portraits = load_all(&palette).unwrap();
    for id in ["test_lord", "test_knight"] {
        let p = &portraits[id];
        assert_eq!(p.character, id);
        for e in REQUIRED_EXPRESSIONS {
            assert!(p.expression(e).is_some(), "{id} lacks {e}");
        }
    }
    // With colours missing from the palette, every file fails.
    let errs = load_all(&PaletteDef::default()).unwrap_err();
    assert!(errs.len() >= portraits.len());
}

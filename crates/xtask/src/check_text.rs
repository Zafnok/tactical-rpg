//! `cargo xtask check-text`: counts the player-facing text still written
//! as string literals in `crates/ui/src`, instead of asked for by key with
//! `ctx.text("screen.thing")` from `assets/lang/en/ui.ron` (ticket 0233,
//! ADR-0045). It fails when the count is above [`MAX_LITERALS`], so no new
//! literal slips in while ticket 0234 moves the old ones out.
//!
//! Like `check-keys` it is a line-based scanner, not a parser, built on
//! the same lexer. A string literal counts when it holds a word (letters
//! outside `{…}` placeholders) and is either
//!
//! - an argument of a call that puts text on the screen (`print…`,
//!   `draw…`, `MenuItem::new`, `MenuItem::disabled`, `with_suffix`,
//!   `help_line`), directly or inside a `format!` there; or
//! - the value of a `&str` constant (or an array of them) that looks like
//!   prose: it has a capital letter or a space. Cue names, storage keys
//!   and screen names (`"menu_move"`) don't.
//!
//! The arguments of `text(…)` and `text_with(…)` are keys and value names,
//! never text. Tests, the test harness and the debug screens are skipped.
//! A literal the rules get wrong is let through with a
//! `// check-text: not player text` comment on its line or the line above
//! its item.

use std::cmp::Ordering;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::check_keys::{
    Line, exempt_lines, files_under, has_ext, is_ident, is_test_file, lex, mark_item, relative,
};

/// The most literals allowed: the count when the title screen had been
/// converted (ticket 0233). Lower it whenever the count goes down; ticket
/// 0234 brings it to zero.
pub const MAX_LITERALS: usize = 221;

/// The comment that lets the next item (or its own line) through.
pub const MARKER: &str = "check-text: not player text";

/// What every failure says to do about it.
pub const HINT: &str =
    "put the text in assets/lang/en/ui.ron and ask for it with `ctx.text(\"screen.thing\")`";

/// The Rust source tree scanned.
const DIR: &str = "crates/ui/src";

/// The debug screens: for developers, in English only.
const DEBUG_FILE: &str = "crates/ui/src/debug.rs";
const DEBUG_DIR: &str = "crates/ui/src/debug/";

/// One literal found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// Repo-relative path of its file.
    pub file: String,
    /// Its 1-based line.
    pub line: usize,
    /// The literal's text.
    pub text: String,
}

/// Scans `repo_root` and returns every literal found, by file then line,
/// and every file that couldn't be read.
pub fn run(repo_root: &Path) -> (Vec<Hit>, Vec<String>) {
    let mut errors = Vec::new();
    let mut hits = Vec::new();
    for file in files_under(&repo_root.join(DIR), &mut errors) {
        let rel = relative(repo_root, &file);
        if !has_ext(&file, "rs") || is_skipped(&rel) {
            continue;
        }
        match fs::read_to_string(&file) {
            Ok(source) => hits.extend(scan(&rel, &source)),
            Err(e) => errors.push(format!("{rel}: cannot read: {e}")),
        }
    }
    (hits, errors)
}

/// Whether the file `rel` is left out: tests and the debug screens.
fn is_skipped(rel: &str) -> bool {
    is_test_file(rel) || rel == DEBUG_FILE || rel.starts_with(DEBUG_DIR)
}

/// What an open bracket belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bracket {
    /// A call that puts its text on the screen.
    Sink,
    /// `text(…)` or `text_with(…)`: keys and value names.
    Key,
    /// Anything else.
    Other,
}

/// What the call `path(` is (`path` as written: `MenuItem::new`, `print`).
fn bracket_of(path: &str) -> Bracket {
    let name = path.rsplit("::").next().unwrap_or(path);
    if name == "text" || name == "text_with" {
        Bracket::Key
    } else if name.starts_with("print")
        || name.starts_with("draw")
        || name == "help_line"
        || name == "with_suffix"
        || path.ends_with("MenuItem::new")
        || path.ends_with("MenuItem::disabled")
    {
        Bracket::Sink
    } else {
        Bracket::Other
    }
}

/// Whether `text` holds a word: a letter outside `{…}` placeholders.
fn has_word(text: &str) -> bool {
    let mut depth = 0_u32;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            c if depth == 0 && c.is_alphabetic() => return true,
            _ => {}
        }
    }
    false
}

/// Whether `text` looks like something a player reads rather than an id:
/// it has a capital letter or a space.
fn is_prose(text: &str) -> bool {
    text.chars().any(|c| c.is_uppercase() || c == ' ')
}

/// For each line, whether it is part of a `const` or `static` item of
/// `&str`s.
fn const_lines(lines: &[Line]) -> Vec<bool> {
    let mut inside = vec![false; lines.len()];
    for (i, line) in lines.iter().enumerate() {
        let code = line.code.trim_start();
        let code = code.strip_prefix("pub ").unwrap_or(code);
        let code = match code.strip_prefix("pub(") {
            Some(rest) => rest.split_once(") ").map_or(code, |(_, rest)| rest),
            None => code,
        };
        let item = code.starts_with("const ") || code.starts_with("static ");
        let of_str = code.contains("&str") || code.contains("&'static str");
        if item && of_str {
            mark_item(lines, i, &mut inside);
        }
    }
    inside
}

/// Scans one Rust file (`rel` is its repo-relative path, for the report).
pub fn scan(rel: &str, source: &str) -> Vec<Hit> {
    let lines = lex(source);
    let exempt = exempt_lines(&lines, Some(MARKER));
    let consts = const_lines(&lines);
    let mut hits = Vec::new();
    let mut open: Vec<Bracket> = Vec::new();
    let mut in_string = false;
    for (i, line) in lines.iter().enumerate() {
        let skipped = exempt.get(i).copied().unwrap_or(false);
        let in_const = consts.get(i).copied().unwrap_or(false);
        let mut strings = line.strings.iter();
        let mut path = String::new();
        for c in line.code.chars() {
            if c == '"' {
                in_string = !in_string;
                path.clear();
                if !in_string {
                    continue;
                }
                let Some(text) = strings.next() else {
                    continue;
                };
                // The innermost call that says what its arguments are.
                let call = open.iter().rev().find(|&&b| b != Bracket::Other);
                let shown = match call {
                    Some(Bracket::Sink) => true,
                    Some(_) => false,
                    None => in_const && is_prose(text),
                };
                if shown && !skipped && has_word(text) {
                    hits.push(Hit {
                        file: rel.to_owned(),
                        line: i + 1,
                        text: text.clone(),
                    });
                }
                continue;
            }
            match c {
                '(' => open.push(bracket_of(&path)),
                '[' | '{' => open.push(Bracket::Other),
                ')' | ']' | '}' => {
                    open.pop();
                }
                _ => {}
            }
            if is_ident(c) || c == ':' {
                path.push(c);
            } else {
                path.clear();
            }
        }
    }
    hits
}

/// The report: the count against `max`, then the literals by file.
pub fn report(hits: &[Hit], max: usize) -> String {
    let mut out = format!(
        "check-text: {} player-facing string literal(s) in {DIR} (at most {max} allowed)\n",
        hits.len()
    );
    let mut file = "";
    for hit in hits {
        if hit.file != file {
            file = &hit.file;
            let count = hits.iter().filter(|h| h.file == file).count();
            // Writing to a `String` can't fail.
            let _ = writeln!(out, "{file}: {count}");
        }
        let _ = writeln!(out, "  {}: {:?}", hit.line, hit.text);
    }
    out
}

/// What the count means against `max`: an error when above it, a note
/// when below it (the number in the tool should come down too).
pub fn verdict(count: usize, max: usize) -> Result<Option<String>, String> {
    match count.cmp(&max) {
        Ordering::Greater => Err(format!(
            "check-text: {count} literals, over the {max} allowed — {HINT}"
        )),
        Ordering::Less => Ok(Some(format!(
            "check-text: {count} literals, under the {max} allowed: lower MAX_LITERALS in \
             crates/xtask/src/check_text.rs to {count}"
        ))),
        Ordering::Equal => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: &str = "crates/ui/src/screens/foo.rs";

    /// The literals found in `source`, as `line: text`.
    fn found(source: &str) -> Vec<String> {
        scan(SCREEN, source)
            .iter()
            .map(|h| format!("{}: {}", h.line, h.text))
            .collect()
    }

    #[test]
    fn literals_passed_to_print_and_the_text_widgets_count() {
        let source = "\
fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
    buf.print(1, 2, \"Hello\", fg, bg);
    buf.print_fg(1, 3, \"again\", fg);
    print_centred(buf, 4, &format!(\"Turn {n} of {max}\"), fg, bg);
    let items = vec![MenuItem::new(\"Retry\"), MenuItem::disabled(\"quit\")];
    let item = MenuItem::new(label).with_suffix(\"(broken)\", UiColor::HpLow);
    let help = help_line(&[(Some(key), \"select\")]);
    draw_dialog(buf, \"Notes\", &lines);
}
";
        assert_eq!(
            found(source),
            [
                "2: Hello",
                "3: again",
                "4: Turn {n} of {max}",
                "5: Retry",
                "5: quit",
                "6: (broken)",
                "7: select",
                "8: Notes",
            ]
        );
        let hits = scan(SCREEN, source);
        assert_eq!(hits[0].file, SCREEN);
        assert_eq!(hits[0].line, 2);
    }

    #[test]
    fn text_by_key_and_other_calls_do_not_count() {
        let source = "\
fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
    print_centred(buf, 4, ctx.text(\"title.new_game\"), fg, bg);
    buf.print(1, 2, &ctx.text_with(\"battle.turn\", &[(\"count\", &n)]), fg, bg);
    let label = ctx.text(\"title.quit\");
    ctx.audio.play_sound(\"menu_move\");
    let s = format!(\"saved as {name}\");
    buf.print(1, 2, &format!(\"{}/{}\", a, b), fg, bg);
    buf.print(1, 2, \" · \", fg, bg);
    buf.print(1, 2, \"\", fg, bg);
    let x = other(\"Words here\");
}
";
        assert_eq!(found(source), Vec::<String>::new());
    }

    #[test]
    fn prose_constants_count_and_ids_do_not() {
        let source = "\
pub const TITLE: &str = \"Visions of Shuyi\";
const QUIT: &str = \"Quit\";
pub(crate) const NOTE: &'static str = \"no saves yet\";
static NAMES: [&str; 2] = [
    \"New Game\",
    \"load\",
];
const CUE: &str = \"menu_move\";
pub const LAYOUT_KEY: &str = \"layout\";
const ROW: i32 = 9;
const SEPARATOR: &str = \" · \";
let local = \"Not A Const\";
";
        assert_eq!(
            found(source),
            [
                "1: Visions of Shuyi",
                "2: Quit",
                "3: no saves yet",
                "5: New Game"
            ]
        );
    }

    #[test]
    fn a_literal_over_several_lines_counts_once_on_its_first_line() {
        let source = "\
fn f(buf: &mut GlyphBuffer) {
    buf.print(
        1,
        2,
        \"one \\
         two\",
        fg,
    );
    buf.print(1, 2, \"after\", fg, bg);
}
";
        assert_eq!(found(source), ["5: one two", "9: after"]);
    }

    #[test]
    fn tests_and_marked_items_are_skipped() {
        let source = "\
// check-text: not player text
const LOG: &str = \"Could not save\";
const SHOWN: &str = \"Could not load\";
fn f(buf: &mut GlyphBuffer) {
    buf.print(1, 2, \"dev only\", fg, bg); // check-text: not player text
}

#[cfg(test)]
mod tests {
    const WANT: &str = \"New Game\";

    #[test]
    fn t() {
        buf.print(1, 2, \"Hello\", fg, bg);
    }
}
";
        assert_eq!(found(source), ["3: Could not load"]);
    }

    #[test]
    fn words_are_letters_outside_placeholders() {
        for text in ["a", "{n} left", "é", "Turn {n}"] {
            assert!(has_word(text), "{text}");
        }
        for text in ["", " · ", "{}/{}", "{name}", "{a}{b} 12", "}{x}"] {
            assert!(!has_word(text), "{text}");
        }
        assert!(is_prose("New"));
        assert!(is_prose("no saves"));
        assert!(!is_prose("menu_move"));
    }

    #[test]
    fn calls_are_told_apart_by_name() {
        for sink in [
            "print",
            "print_fg",
            "print_centred",
            "draw_tip",
            "map_menu::draw_dialog_at",
            "help_line",
            "with_suffix",
            "MenuItem::new",
            "widgets::MenuItem::disabled",
        ] {
            assert!(bracket_of(sink) == Bracket::Sink, "{sink}");
        }
        for key in ["text", "text_with", "Ctx::text"] {
            assert!(bracket_of(key) == Bracket::Key, "{key}");
        }
        for other in ["", "new", "Menu::new", "format", "context", "reprint"] {
            assert!(bracket_of(other) == Bracket::Other, "{other}");
        }
    }

    #[test]
    fn the_report_lists_the_literals_by_file() {
        let hit = |file: &str, line, text: &str| Hit {
            file: file.to_owned(),
            line,
            text: text.to_owned(),
        };
        let hits = [
            hit("a.rs", 3, "One"),
            hit("a.rs", 9, "Two"),
            hit("b.rs", 1, "x y"),
        ];
        assert_eq!(
            report(&hits, 5),
            "check-text: 3 player-facing string literal(s) in crates/ui/src (at most 5 allowed)\n\
             a.rs: 2\n  3: \"One\"\n  9: \"Two\"\nb.rs: 1\n  1: \"x y\"\n"
        );
    }

    #[test]
    fn one_literal_too_many_fails() {
        let before = "fn f(b: &mut GlyphBuffer) {\n    b.print(0, 0, \"Old\", fg, bg);\n}\n";
        let max = scan(SCREEN, before).len();
        assert_eq!(max, 1);
        assert_eq!(verdict(max, max), Ok(None));
        let after = format!("{before}const NEW: &str = \"New Game\";\n");
        let count = scan(SCREEN, &after).len();
        assert_eq!(count, 2);
        let error = verdict(count, max).unwrap_err();
        assert!(error.contains("2 literals, over the 1 allowed"), "{error}");
        assert!(error.contains(HINT), "{error}");
        // Fewer than allowed passes, with a note to lower the number.
        let note = verdict(0, max).unwrap().unwrap();
        assert!(note.contains("lower MAX_LITERALS"), "{note}");
        assert!(note.contains("to 0"), "{note}");
    }

    #[test]
    fn run_scans_ui_sources_but_not_tests_or_debug_screens() {
        let root = std::env::temp_dir().join(format!("xtask-check-text-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let src = root.join("crates/ui/src");
        fs::create_dir_all(src.join("screens")).unwrap();
        fs::create_dir_all(src.join("debug")).unwrap();
        let literal = "const A: &str = \"Some Text\";\n";
        for file in [
            "screens/foo.rs",
            "screens/tests.rs",
            "debug.rs",
            "debug/menu.rs",
            "harness.rs",
            "screens/notes.txt",
        ] {
            fs::write(src.join(file), literal).unwrap();
        }
        let (hits, errors) = run(&root);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(errors, Vec::<String>::new());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].file, "crates/ui/src/screens/foo.rs");
        // A missing tree is reported, not passed over.
        let (hits, errors) = run(&root);
        assert!(hits.is_empty());
        assert_eq!(errors.len(), 1, "{errors:?}");
    }

    fn repo_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask is at <repo>/crates/xtask")
    }

    #[test]
    fn the_real_repo_is_within_the_allowed_count() {
        let (hits, errors) = run(repo_root());
        assert_eq!(errors, Vec::<String>::new());
        assert!(
            hits.len() <= MAX_LITERALS,
            "{}",
            report(&hits, MAX_LITERALS)
        );
    }

    /// Ticket 0233: the title screen is the first one converted.
    #[test]
    fn the_title_screen_has_no_literal_left() {
        let (hits, _) = run(repo_root());
        let title: Vec<&Hit> = hits
            .iter()
            .filter(|h| h.file == "crates/ui/src/screens/title.rs")
            .collect();
        assert!(title.is_empty(), "{title:?}");
    }
}

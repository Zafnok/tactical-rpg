//! The `.dlg` parser: lines → scenes. Reports syntax and structure errors
//! (unknown directives, bad ids, missing `@end`, non-ASCII text); checks
//! that need the whole scene or other files are in `check`.

use trpg_core::CharacterId;

use super::{Scene, Side, Step};
use crate::error::ContentError;

/// A scene as written in a file, with the source lines the checks point at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedScene {
    /// The scene.
    pub scene: Scene,
    /// The file it is in (for error messages).
    pub file: String,
    /// 1-based line of its `@scene`.
    pub line: u32,
    /// 1-based line where each step starts (same order as `scene.steps`).
    pub step_lines: Vec<u32>,
}

/// Parses dialogue `source` (errors attributed to `file`) into its scenes,
/// in file order, and every syntax error found. A scene with errors in it
/// is still returned (without the broken lines), so later checks can run.
pub fn parse_dlg(file: &str, source: &str) -> (Vec<ParsedScene>, Vec<ContentError>) {
    let mut p = Parser {
        file,
        scenes: Vec::new(),
        open: None,
        continuable: false,
        errors: Vec::new(),
    };
    for (i, line) in source.split('\n').enumerate() {
        p.line(u32::try_from(i + 1).unwrap_or(u32::MAX), line);
    }
    if let Some(open) = p.open.take() {
        p.no_end(&open);
        p.scenes.push(open);
    }
    (p.scenes, p.errors)
}

/// Whether `s` is a valid scene, character or expression id: lowercase
/// ASCII letters, digits and `_`, not empty.
pub(crate) fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// What is wrong with character `c` in dialogue text, if anything. Text
/// must be printable ASCII (the font may lack anything else); common
/// typographic punctuation gets its ASCII spelling suggested.
pub(crate) fn char_problem(c: char) -> Option<String> {
    if (' '..='~').contains(&c) {
        return None;
    }
    let ascii = match c {
        '\u{2018}' | '\u{2019}' => "'",
        '\u{201C}' | '\u{201D}' => "\"",
        '\u{2026}' => "...",
        '\u{2014}' => "--",
        '\u{2013}' => "-",
        _ => {
            return Some(format!(
                "unsupported character {c:?}; dialogue text is plain ASCII"
            ));
        }
    };
    Some(format!("non-ASCII punctuation {c:?}; use {ascii} instead"))
}

struct Parser<'a> {
    file: &'a str,
    scenes: Vec<ParsedScene>,
    /// The scene being read (after `@scene`, before `@end`).
    open: Option<ParsedScene>,
    /// Whether the last line was speech or narration, so an indented line
    /// continues it.
    continuable: bool,
    errors: Vec<ContentError>,
}

impl Parser<'_> {
    fn err(&mut self, line: u32, message: impl Into<String>) {
        self.errors
            .push(ContentError::new(self.file, message).at(line, None));
    }

    fn no_end(&mut self, open: &ParsedScene) {
        self.err(
            open.line,
            format!("scene \"{}\" has no @end", open.scene.id),
        );
    }

    /// Reads line `n` (1-based).
    fn line(&mut self, n: u32, raw: &str) {
        let line = raw.trim_end();
        let body = line.trim_start_matches(' ');
        if body.is_empty() || body.starts_with('#') {
            return;
        }
        if body.len() < line.len() {
            self.continuation(n, line, body);
            return;
        }
        self.continuable = false;
        if let Some(rest) = body.strip_prefix('@') {
            self.directive(n, line, rest);
        } else if let Some(rest) = body.strip_prefix('>') {
            let text = rest.trim();
            if text.is_empty() {
                self.err(n, "narration has no text");
            } else {
                self.check_chars(n, line, text);
                self.push(n, Step::Narrate { text: text.into() });
                self.continuable = self.open.is_some();
            }
        } else {
            self.speech(n, line);
        }
    }

    /// An indented line: more text for the speech or narration above.
    fn continuation(&mut self, n: u32, line: &str, body: &str) {
        if body.starts_with('@') {
            self.err(n, "directives can't be indented");
            return;
        }
        let last = self
            .open
            .as_mut()
            .filter(|_| self.continuable)
            .and_then(|s| s.scene.steps.last_mut());
        match last {
            Some(Step::Say { text, .. } | Step::Narrate { text }) => {
                text.push(' ');
                text.push_str(body);
                self.check_chars(n, line, body);
            }
            _ => self.err(
                n,
                "indented line doesn't continue a speech or narration line",
            ),
        }
    }

    /// `@<rest>` on line `n`.
    fn directive(&mut self, n: u32, line: &str, rest: &str) {
        let (name, args) = rest
            .split_once(' ')
            .map_or((rest, ""), |(name, args)| (name, args.trim()));
        match name {
            "scene" => self.scene(n, args),
            "end" => self.end(n, args),
            "caption" if args.is_empty() => self.err(n, "@caption has no text"),
            "caption" => {
                self.check_chars(n, line, args);
                self.push(n, Step::Caption { text: args.into() });
            }
            "left" => self.place(n, Side::Left, args),
            "right" => self.place(n, Side::Right, args),
            _ => self.err(n, format!("unknown directive \"@{name}\"")),
        }
    }

    fn scene(&mut self, n: u32, args: &str) {
        if let Some(open) = self.open.take() {
            self.no_end(&open);
            self.scenes.push(open);
        }
        let ok = args.split_whitespace().count() == 1;
        if !ok {
            self.err(n, "@scene needs one scene id");
        } else if !is_id(args) {
            self.bad_id(n, args);
        }
        self.open = Some(ParsedScene {
            scene: Scene {
                id: args.into(),
                steps: Vec::new(),
            },
            file: self.file.into(),
            line: n,
            step_lines: Vec::new(),
        });
    }

    fn end(&mut self, n: u32, args: &str) {
        if !args.is_empty() {
            self.err(n, "@end takes nothing after it");
        }
        match self.open.take() {
            Some(open) => self.scenes.push(open),
            None => self.err(n, "@end without an open @scene"),
        }
    }

    /// `@left`/`@right` with `args`: `<id> <expression>` or `clear`.
    fn place(&mut self, n: u32, side: Side, args: &str) {
        let args: Vec<&str> = args.split_whitespace().collect();
        let step = match args[..] {
            ["clear"] => Step::Clear { side },
            [character, expression] if character != "clear" => {
                if !is_id(character) {
                    self.bad_id(n, character);
                }
                if !is_id(expression) {
                    self.bad_id(n, expression);
                }
                Step::Place {
                    side,
                    character: CharacterId(character.into()),
                    expression: expression.into(),
                }
            }
            _ => {
                self.err(
                    n,
                    format!(
                        "@{} needs a character id and an expression, or \"clear\"",
                        side.name()
                    ),
                );
                return;
            }
        };
        self.push(n, step);
    }

    /// `id: text` or `id[expression]: text` on line `n`.
    fn speech(&mut self, n: u32, line: &str) {
        let Some((head, text)) = line.split_once(':').filter(|(h, _)| !h.contains(' ')) else {
            self.err(
                n,
                "unrecognised line; expected \"@directive\", \"> narration\", \"speaker: text\" or \"# comment\"",
            );
            return;
        };
        let (speaker, expression) = match head.strip_suffix(']').and_then(|h| h.split_once('[')) {
            Some((speaker, expression)) => (speaker, Some(expression)),
            None => (head, None),
        };
        for id in std::iter::once(speaker).chain(expression) {
            if !is_id(id) {
                self.bad_id(n, id);
            }
        }
        let text = text.trim();
        if text.is_empty() {
            self.err(n, format!("\"{head}:\" has no text"));
            return;
        }
        self.check_chars(n, line, text);
        self.push(
            n,
            Step::Say {
                speaker: CharacterId(speaker.into()),
                expression: expression.map(Into::into),
                text: text.into(),
            },
        );
        self.continuable = self.open.is_some();
    }

    fn bad_id(&mut self, n: u32, id: &str) {
        self.err(
            n,
            format!("\"{id}\" is not a valid id; use lowercase letters, digits and _"),
        );
    }

    /// Adds `step` (from line `n`) to the open scene.
    fn push(&mut self, n: u32, step: Step) {
        match self.open.as_mut() {
            Some(open) => {
                open.scene.steps.push(step);
                open.step_lines.push(n);
            }
            None => self.err(n, "line is outside a scene; start one with @scene <id>"),
        }
    }

    /// Reports every unsupported character of `text`, a suffix of `line`,
    /// with its column.
    fn check_chars(&mut self, n: u32, line: &str, text: &str) {
        let before = line[..line.len() - text.len()].chars().count();
        for (i, c) in text.chars().enumerate() {
            if let Some(message) = char_problem(c) {
                let column = u32::try_from(before + i + 1).unwrap_or(u32::MAX);
                self.errors
                    .push(ContentError::new(self.file, message).at(n, Some(column)));
            }
        }
    }
}

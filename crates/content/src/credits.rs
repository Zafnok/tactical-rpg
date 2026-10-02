//! The credits (ticket 0808; `docs/design/audio.md` rule 3): every
//! third-party work the game uses, for the credits screen. Music and sounds
//! are credited in the audio manifest ([`crate::audio`]); everything else
//! (the font, vendored software) in `assets/data/credits.ron`. [`load`]
//! merges the two into one list, so nothing is typed twice.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::audio::{self, AudioManifest};
use crate::bundle;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the credits file inside the asset bundle.
pub const CREDITS_PATH: &str = "data/credits.ron";

/// Licenses an entry of the credits file may have: ADR-0013's code and
/// font lists.
pub const LICENSES: [&str; 15] = [
    "MIT",
    "MIT-0",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "0BSD",
    "ISC",
    "Zlib",
    "BSL-1.0",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "Unlicense",
    "OFL-1.1",
];

/// The kinds of work the credits screen groups by, in the order it shows
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub enum CreditGroup {
    /// Music tracks (from the audio manifest's music cues).
    Music,
    /// Sound effects (from the audio manifest's sound cues).
    SoundEffects,
    /// Fonts.
    Fonts,
    /// Vendored code and data tables.
    Software,
}

impl CreditGroup {
    /// Every group, in display order.
    pub const ALL: [Self; 4] = [Self::Music, Self::SoundEffects, Self::Fonts, Self::Software];

    /// Whether the group's credits come from the audio manifest, so the
    /// credits file may not use it.
    pub fn is_audio(self) -> bool {
        matches!(self, Self::Music | Self::SoundEffects)
    }
}

/// One entry of the credits file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCredit {
    /// Stable id, unique in the file.
    pub id: String,
    /// What kind of work it is; not an audio group.
    pub group: CreditGroup,
    /// The work's title.
    pub title: String,
    /// Its author, as they want to be credited.
    pub author: String,
    /// The page it came from: the link in its `THIRD_PARTY_ASSETS.md` row.
    pub source: String,
    /// One of [`LICENSES`].
    pub license: String,
    /// Kept off the credits screen: plumbing the player never sees.
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    credits: Vec<FileCredit>,
}

/// The validated credits file, in file order, hidden entries included.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreditsFile {
    /// The entries.
    pub credits: Vec<FileCredit>,
}

/// One work as the credits screen shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditEntry {
    /// The heading it goes under.
    pub group: CreditGroup,
    /// The work's title.
    pub title: String,
    /// Its author.
    pub author: String,
    /// Its license (an SPDX id, e.g. `CC-BY-4.0`).
    pub license: String,
    /// The page it came from.
    pub source: String,
}

/// Every credit the screen shows: sorted by group ([`CreditGroup::ALL`]),
/// audio works by title within their group, the file's in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Credits {
    /// The entries, in display order.
    pub entries: Vec<CreditEntry>,
}

impl Credits {
    /// The entries under `group`, in display order.
    pub fn in_group(&self, group: CreditGroup) -> impl Iterator<Item = &CreditEntry> {
        self.entries.iter().filter(move |e| e.group == group)
    }
}

/// Loads and validates the embedded credits file.
pub fn load_file() -> Result<CreditsFile, Vec<ContentError>> {
    let display = bundle::display_path(CREDITS_PATH);
    let source = bundle::file(CREDITS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source)
}

/// Loads the embedded credits file and merges it with `audio`'s credits.
/// Without the manifest (it failed to load) the list has no audio credits.
pub fn load(audio: Option<&AudioManifest>) -> Result<Credits, Vec<ContentError>> {
    let file = load_file()?;
    Ok(merge(&file, audio.unwrap_or(&AudioManifest::default())))
}

/// Parses and validates credits `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(file: &str, source: &str) -> Result<CreditsFile, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    for c in &raw.credits {
        let mut err = |message: String| {
            let e = ContentError::new(file, format!("credit \"{}\": {message}", c.id));
            errors.push(match line_of(source, &format!("id: \"{}\"", c.id)) {
                Some(l) => e.at(l, None),
                None => e,
            });
        };
        if c.id.is_empty() {
            err("the id is empty".into());
        }
        if !ids.insert(c.id.as_str()) {
            err("the id is used twice".into());
        }
        if c.group.is_audio() {
            err(format!(
                "{:?} credits go in {}",
                c.group,
                bundle::display_path(audio::AUDIO_PATH)
            ));
        }
        for (field, value) in [("title", &c.title), ("author", &c.author)] {
            if value.trim().is_empty() {
                err(format!("{field} is missing"));
            }
        }
        if !c.source.starts_with("https://") && !c.source.starts_with("http://") {
            err("source must be a web link (http:// or https://)".into());
        }
        if !LICENSES.contains(&c.license.as_str()) {
            err(format!(
                "license \"{}\" is not allowed (ADR-0013: {})",
                c.license,
                LICENSES.join(", ")
            ));
        }
    }
    if errors.is_empty() {
        Ok(CreditsFile {
            credits: raw.credits,
        })
    } else {
        Err(errors)
    }
}

/// The credits the screen shows: the third-party works `audio`'s music
/// cues name, then those its sound cues name (each once, by title; a work
/// used for both counts as music), then `file`'s entries that aren't
/// hidden. Our own work ([`audio::OWN`]) isn't listed.
pub fn merge(file: &CreditsFile, audio: &AudioManifest) -> Credits {
    let music = audio.music.values().map(|m| &m.credit);
    let sounds = audio.sounds.values().map(|s| &s.credit);
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for (group, refs) in [
        (CreditGroup::Music, music.collect::<Vec<_>>()),
        (CreditGroup::SoundEffects, sounds.collect()),
    ] {
        let from = entries.len();
        for id in refs.into_iter().flat_map(audio::CreditRef::ids) {
            let Some(c) = audio.credits.get(id) else {
                continue;
            };
            if c.license != audio::OWN && seen.insert(id) {
                entries.push(CreditEntry {
                    group,
                    title: c.title.clone(),
                    author: c.author.clone(),
                    license: c.license.clone(),
                    source: c.source.clone(),
                });
            }
        }
        entries[from..].sort_by_key(|e| e.title.to_lowercase());
    }
    let shown = file.credits.iter().filter(|c| !c.hidden);
    entries.extend(shown.map(|c| CreditEntry {
        group: c.group,
        title: c.title.clone(),
        author: c.author.clone(),
        license: c.license.clone(),
        source: c.source.clone(),
    }));
    // Stable: keeps the orders above within each group.
    entries.sort_by_key(|e| e.group);
    Credits { entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Credit, CreditRef, MusicCue, SoundCue};

    const FILE: &str = r#"(credits: [
    (id: "font", group: Fonts, title: "A Font", author: "Ann",
     source: "https://example.org/font", license: "OFL-1.1"),
    (id: "loader", group: Software, title: "Loader", author: "Bob",
     source: "http://example.org/loader", license: "MIT", hidden: true),
    (id: "table", group: Software, title: "Table", author: "Cy",
     source: "https://example.org/table", license: "Zlib"),
])"#;

    fn errors(source: &str) -> Vec<String> {
        from_source("c.ron", source)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn credit(title: &str, license: &str) -> Credit {
        Credit {
            title: title.into(),
            author: format!("{title}'s author"),
            source: format!("https://example.org/{title}"),
            license: license.into(),
            ..Credit::default()
        }
    }

    fn music(credit: CreditRef) -> MusicCue {
        MusicCue {
            file: "a.ogg".into(),
            volume: 100,
            looped: true,
            length_ms: 1000,
            credit,
        }
    }

    fn sound(credit: CreditRef) -> SoundCue {
        SoundCue {
            files: vec!["a.ogg".into()],
            volume: 100,
            credit,
        }
    }

    /// Two tracks, three sounds (one ours, one sharing a track's work).
    fn manifest() -> AudioManifest {
        let one = |id: &str| CreditRef::Credit(id.into());
        let mut m = AudioManifest::default();
        for (id, c) in [
            ("zeta", credit("Zeta", "CC0-1.0")),
            ("alpha", credit("alpha", "CC-BY-4.0")),
            ("step", credit("Step", "CC-BY-3.0")),
            ("clang", credit("Clang", "CC0-1.0")),
            ("ours", credit("", audio::OWN)),
            ("unused", credit("Unused", "CC0-1.0")),
        ] {
            m.credits.insert(id.into(), c);
        }
        m.music.insert("a".into(), music(one("zeta")));
        m.music.insert("b".into(), music(one("alpha")));
        m.music.insert("c".into(), music(one("zeta")));
        m.music.insert("d".into(), music(CreditRef::Own));
        m.music.insert("e".into(), music(one("ours")));
        m.sounds.insert(
            "s".into(),
            sound(CreditRef::Credits(vec!["step".into(), "clang".into()])),
        );
        m.sounds.insert("t".into(), sound(one("zeta")));
        m.sounds.insert("u".into(), sound(CreditRef::Own));
        m.sounds.insert("v".into(), sound(one("missing")));
        m
    }

    fn shown(credits: &Credits) -> Vec<(CreditGroup, &str)> {
        credits
            .entries
            .iter()
            .map(|e| (e.group, e.title.as_str()))
            .collect()
    }

    #[test]
    fn a_good_file_loads_in_file_order() {
        let file = from_source("c.ron", FILE).unwrap_or_default();
        let ids: Vec<&str> = file.credits.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["font", "loader", "table"]);
        let hidden: Vec<bool> = file.credits.iter().map(|c| c.hidden).collect();
        assert_eq!(hidden, [false, true, false]);
        assert_eq!(file.credits[0].group, CreditGroup::Fonts);
        assert_eq!(file.credits[0].license, "OFL-1.1");
        assert_eq!(
            from_source("c.ron", "(credits: [])"),
            Ok(CreditsFile::default())
        );
    }

    #[test]
    fn bad_entries_are_reported_with_their_line() {
        let dup = FILE.replace(r#"id: "table""#, r#"id: "font""#);
        assert_eq!(
            errors(&dup),
            ["c.ron:2: credit \"font\": the id is used twice"]
        );
        let empty = FILE.replace(r#"id: "table""#, r#"id: """#);
        assert_eq!(errors(&empty), ["c.ron:6: credit \"\": the id is empty"]);
        let bare = FILE
            .replace(
                r#"title: "Table", author: "Cy""#,
                r#"title: " ", author: """#,
            )
            .replace("https://example.org/table", "example.org/table");
        assert_eq!(
            errors(&bare),
            [
                "c.ron:6: credit \"table\": title is missing",
                "c.ron:6: credit \"table\": author is missing",
                "c.ron:6: credit \"table\": source must be a web link (http:// or https://)",
            ]
        );
        let gpl = FILE.replace("\"Zlib\"", "\"GPL-3.0\"");
        assert_eq!(
            errors(&gpl),
            [format!(
                "c.ron:6: credit \"table\": license \"GPL-3.0\" is not allowed (ADR-0013: {})",
                LICENSES.join(", ")
            )]
        );
    }

    #[test]
    fn audio_groups_belong_to_the_audio_manifest() {
        assert_eq!(
            errors(&FILE.replace("group: Fonts", "group: Music")),
            ["c.ron:2: credit \"font\": Music credits go in assets/audio/audio.ron"]
        );
        assert_eq!(
            errors(&FILE.replace("group: Fonts", "group: SoundEffects")),
            ["c.ron:2: credit \"font\": SoundEffects credits go in assets/audio/audio.ron"]
        );
        let audio: Vec<bool> = CreditGroup::ALL.iter().map(|g| g.is_audio()).collect();
        assert_eq!(audio, [true, true, false, false]);
    }

    #[test]
    fn syntax_and_unknown_fields_are_refused() {
        assert_eq!(errors("(credits: [").len(), 1);
        assert_eq!(errors(&FILE.replace("hidden: true", "hide: true")).len(), 1);
        assert_eq!(errors(&FILE.replace("group: Fonts", "group: Art")).len(), 1);
    }

    #[test]
    fn merging_groups_sorts_and_drops_what_is_not_shown() {
        use CreditGroup::{Fonts, Music, Software, SoundEffects};
        let file = from_source("c.ron", FILE).unwrap_or_default();
        let credits = merge(&file, &manifest());
        // Music by title whatever its case, each work once; a work used by
        // a track and a sound is music; ours, the unused credit, the
        // unknown id and the hidden entry aren't shown.
        assert_eq!(
            shown(&credits),
            [
                (Music, "alpha"),
                (Music, "Zeta"),
                (SoundEffects, "Clang"),
                (SoundEffects, "Step"),
                (Fonts, "A Font"),
                (Software, "Table"),
            ]
        );
        let first = &credits.entries[0];
        assert_eq!(first.author, "alpha's author");
        assert_eq!(first.license, "CC-BY-4.0");
        assert_eq!(first.source, "https://example.org/alpha");
        let font = credits.in_group(Fonts).collect::<Vec<_>>();
        assert_eq!(font.len(), 1);
        assert_eq!(
            (font[0].author.as_str(), font[0].license.as_str()),
            ("Ann", "OFL-1.1")
        );
        assert_eq!(font[0].source, "https://example.org/font");
        assert_eq!(credits.in_group(Music).count(), 2);
    }

    #[test]
    fn file_entries_keep_their_order_and_sort_under_their_group() {
        use CreditGroup::{Fonts, Software};
        // Software before Fonts in the file: Fonts still shows first, and
        // two entries of one group stay in file order.
        let source = r#"(credits: [
            (id: "z", group: Software, title: "Zed", author: "a", source: "https://z", license: "MIT"),
            (id: "b", group: Software, title: "Bee", author: "a", source: "https://b", license: "MIT"),
            (id: "f", group: Fonts, title: "Font", author: "a", source: "https://f", license: "MIT"),
        ])"#;
        let file = from_source("c.ron", source).unwrap_or_default();
        let credits = merge(&file, &AudioManifest::default());
        assert_eq!(
            shown(&credits),
            [(Fonts, "Font"), (Software, "Zed"), (Software, "Bee")]
        );
    }

    #[test]
    fn the_embedded_file_loads_and_merges_with_the_audio_manifest() {
        let file = load_file();
        assert!(file.is_ok(), "{file:?}");
        let audio = audio::load().unwrap_or_default();
        let credits = load(Some(&audio)).unwrap_or_default();
        // Every third-party work in the manifest is shown, once.
        let mut want: Vec<&str> = audio
            .credits
            .values()
            .filter(|c| c.license != audio::OWN)
            .map(|c| c.source.as_str())
            .collect();
        let mut got: Vec<&str> = credits
            .entries
            .iter()
            .filter(|e| e.group.is_audio())
            .map(|e| e.source.as_str())
            .collect();
        want.sort_unstable();
        got.sort_unstable();
        assert_eq!(got, want);
        assert!(credits.in_group(CreditGroup::Music).count() > 15);
        assert!(credits.in_group(CreditGroup::SoundEffects).count() > 10);
        let fonts: Vec<&str> = credits
            .in_group(CreditGroup::Fonts)
            .map(|e| e.title.as_str())
            .collect();
        assert_eq!(fonts, ["Terminus Font"]);
        // Without the manifest: the file's entries alone.
        let alone = load(None).unwrap_or_default();
        assert!(!alone.entries.is_empty());
        assert!(alone.entries.iter().all(|e| !e.group.is_audio()));
    }

    /// The first link in each row's "Source URL" cell of
    /// `THIRD_PARTY_ASSETS.md`'s table.
    fn asset_sources() -> Vec<String> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../THIRD_PARTY_ASSETS.md");
        let doc = std::fs::read_to_string(path).unwrap_or_default();
        doc.lines()
            .filter(|l| l.starts_with("| ") && !l.starts_with("| Item ") && !l.starts_with("| -"))
            .map(|row| {
                let cell = row.split('|').nth(2).unwrap_or_default();
                let link = cell.split_whitespace().find(|w| w.starts_with("http"));
                link.unwrap_or_default().trim_end_matches(',').to_owned()
            })
            .collect()
    }

    /// Every work in `THIRD_PARTY_ASSETS.md` has a credit, in the credits
    /// file or the audio manifest, and every credit has a row there, so
    /// the credits screen can't fall out of date (ticket 0808). They are
    /// matched by source link.
    #[test]
    fn every_third_party_asset_has_a_credit() {
        let rows = asset_sources();
        assert!(rows.len() > 40, "the table wasn't found: {rows:?}");
        let file = load_file().unwrap_or_default();
        let audio = audio::load().unwrap_or_default();
        let credited: BTreeSet<&str> = file
            .credits
            .iter()
            .map(|c| c.source.as_str())
            .chain(
                audio
                    .credits
                    .values()
                    .filter(|c| c.license != audio::OWN)
                    .map(|c| c.source.as_str()),
            )
            .collect();
        let listed: BTreeSet<&str> = rows.iter().map(String::as_str).collect();
        let no_credit: Vec<&&str> = listed.difference(&credited).collect();
        assert!(
            no_credit.is_empty(),
            "in THIRD_PARTY_ASSETS.md without a credit: {no_credit:?}"
        );
        let no_row: Vec<&&str> = credited.difference(&listed).collect();
        assert!(
            no_row.is_empty(),
            "credited but not in THIRD_PARTY_ASSETS.md: {no_row:?}"
        );
        // Hidden entries are software only: every font is shown.
        for c in file.credits.iter().filter(|c| c.hidden) {
            assert_eq!(c.group, CreditGroup::Software, "{} is hidden", c.id);
        }
    }
}

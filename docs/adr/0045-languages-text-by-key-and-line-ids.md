# ADR-0045: Languages: text by key, dialogue lines by id, wide glyphs in two cells

- **Status:** Accepted (§1–2 built by ticket 0233, §3 by ticket 0717; the details each settled are marked *0233* and *As built (ticket 0717)*)
- **Date:** 2026-10-03
- **Related tickets:** 0042, 0233, 0234, 0235, 0236, 0237, 0717, 0718, 0719, 0723, 0825, 0907
- **Amends:** ADR-0016 (the atlas gains 16×16 glyphs from a second font) and
  ADR-0011 (a translation step after the script). Nothing in either is
  superseded.

## Context

Nick wants a Japanese option, machine-translated first and replaced by a
human translation if the game earns enough; and the English script itself
may later be rewritten by a hired writer
(`docs/design/voices-languages-and-script.md`). So text must be replaceable
piece by piece, and it must be plain which pieces are out of date.

Facts today:

- Player-facing English sits in four places: string literals in `crates/ui`
  (about 200), `name:` and description fields in `assets/data/*.ron`, the
  names table `assets/data/names.ron`, and `assets/dialogue/*.dlg`.
- A `GlyphBuffer` cell holds one `char`, 8×16 px, from one font (Terminus,
  no Japanese). The console is 100×32 cells.
- Dialogue lines have no ids; scenes do. The validator requires plain ASCII
  and counts characters for its length limits.
- Line wrapping breaks at spaces. Japanese has none.
- Nobody on the project reads Japanese.

## Decision

### 1. English is the source; other languages are packs that overlay it

`assets/lang/<code>/` holds one language (`ja`). A pack has:

- `lang.ron`: the language's own name (`日本語`), and `made_by: Machine` or
  `Human(credit)`.
- `ui.ron`, `data.ron`, `dialogue/<file>.ron`: lists of entries
  `(key, source, text)`. `source` is a copy of the English the translation
  was made from.

Loaded and validated by `trpg_content::lang` into `Content`. An entry whose
key doesn't exist is an error. An entry whose `source` differs from today's
English is **stale**; a key with no entry is **missing**. Stale and missing
text shows in English and never fails a gate, so editing English (or a
writer rewriting a scene) can't block a PR. `cargo xtask lang-status <code>`
lists both; a release ticket requires the list to be empty or accepted.

*0233:* English's `ui.ron` is a map of key to text; a pack's is a list of
`(key: …, source: …, text: …)`. A language code is 2 to 8 lowercase
letters. A pack entry's placeholders must be the same set as its
**`source`'s** (not today's English's), so that changing English can only
make an entry stale, never fail a gate. Until 0235 adds `data.ron` and
`dialogue/`, any other file in a language's directory is an error.
`assets/lang/test/` is a pack for tests (English in capitals, with one
stale and one missing entry kept on purpose); it is in the bundle but
only counts where debug tools are on, and is never offered to players.

### 2. Keys

| Text | Key | English lives in |
| ---- | --- | ---------------- |
| Screen text | chosen by hand, `screen.thing` (`title.new_game`) | `assets/lang/en/ui.ron` |
| Data names and descriptions | `<file>.<id>.<field>` (`items.iron_sword.name`) | the data file, as now |
| Names table | `names.<id>` | `names.ron`, as now |
| Tips | `tips.<id>.title`, `tips.<id>.text` | `tips.ron`, as now |
| Dialogue | the line id (below) | the `.dlg` file, as now |

Screens ask for text with `ctx.text("title.new_game")`; values use named
placeholders (`{count}`), filled the way tips fill `{Confirm}`
(*0233:* `ctx.text_with(key, &[("count", &n)])` fills both in one pass, so
a help line is one text, `{Cursor} move · {Confirm} select`, and a
translation can reorder it). `cargo xtask check-text` counts the screen
text still written as literals in `crates/ui/src` and fails on a new one. A key the
English file lacks panics in debug builds, like an unknown audio cue
(ADR-0026), so a typo fails a test. Only `ui` and `content` know about
languages; `core` holds no player-facing text.

### 3. Dialogue line ids

Every speech line, narration line and reply gets an id computed by
`trpg_content::dialogue`, not written in the script:

`<scene id>_<8 hex digits>`: a hash of the speaker id and the line's English
text as written (tokens unexpanded, continuation lines joined). A second
identical line in the same scene gets `_2`, and so on. The hash is a fixed
algorithm written in the crate (FNV-1a 64, truncated), never `std`'s hasher.

As built (ticket 0717): the hash is over the speaker, a newline and the
text, where the speaker is the character id, `>` for narration or `*` for a
reply, and continuation lines are joined with one space; the id keeps the
low 32 bits. The lines of a reply's reaction get ids like any other. The
speaker's expression and a reply's tone are not hashed. Two *different*
lines of one scene with the same hash are a validation error (reword one):
numbering them would make their ids depend on their order.
`cargo xtask lines [scene]` lists every line with its id.

This is how Ren'Py keys translations and automatic voice. It keeps scripts
free of id clutter, moving or inserting lines changes nothing, and changing
a line's words gives it a new id, which is exactly when its translation and
its voice clip (ADR-0046) need redoing. `lang-status` pairs an orphaned
entry with the new line nearest to it so a translator sees the old text.

### 4. Japanese glyphs are 16×16 and take two cells

- The atlas gains glyphs from a second, 16×16 font (picked by Nick in 0042
  from fonts ADR-0013 allows). Only the characters the language packs use
  are included; `cargo xtask font-atlas` collects them, and the stale-atlas
  test covers it, so a translation PR regenerates the atlas.
- `FontAtlasDef` says which glyphs are wide. A wide glyph is printed into
  the first of two cells; the second is a continuation cell. Widths come
  from the atlas, so text measuring in `ui` and length checks in `content`
  agree with what is drawn.
- Length limits count cells, not characters. Wrapping for Japanese breaks
  between any two characters, except before closing punctuation
  (`。、」` and the like) and after opening brackets.
- The "plain ASCII" rule stays for English only.

### 5. The player's language

`Settings` (0805) holds the language code; `Ctx` carries it. How the player
picks it is Nick's (0042). Saves don't store text, so a save loads in any
language.

### 6. Translating

Translation is a step after the script in ADR-0011's pipeline, done by
Claude per chapter (0718 sets it up): a glossary of game terms and names, a
speech-style sheet per character (first-person word, politeness, sentence
endings), then translate, then a separate pass that translates back to
English and compares. Because nobody here can read the result, the game and
the store page say the translation is machine-made (0042, 0907).

## Consequences

- A human translator later edits the same pack files and flips `made_by`.
- A hired scriptwriter edits `.dlg` files; changed lines turn stale in
  every pack and lose their voice clip, and `lang-status` and the voice
  tool list them. Nothing else needs touching.
- Moving ~200 literals into `ui.ron` touches every screen once (0233, 0234).
- Renaming something in `names.ron` makes its translation stale, as it
  should.
- Text-width code can no longer assume one `char` is one cell.
- Other languages with wide or accented glyphs fit the same scheme; a
  right-to-left one would not.

## Alternatives considered

- **Ids written in the script** (Yarn Spinner's `#line:` tags) — stable
  through rewording, but a tool must stamp them, they double the noise in a
  file meant for a human writer, and a reworded line would keep a voice
  clip that says the old words.
- **`<scene>` + line number as the id** — every inserted line would shift
  the ids after it.
- **gettext / Fluent crates** — more machinery than a keyed table needs,
  and their plural rules matter little for Japanese; another dependency to
  licence-check.
- **English text as the key** — the same word needs different translations
  in different places.
- **A whole translated copy of each `.dlg`** — directives (music, portraits,
  choices) would be duplicated and drift apart.
- **A TTF renderer for Japanese** — loses the pixel look (ADR-0016).
- **All of JIS X 0208 in the atlas** — about 7,000 glyphs nobody uses.

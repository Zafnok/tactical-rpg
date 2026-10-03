---
id: "0811"
title: Title screen art with the game's name
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0012", "0801"]
nick_input: decision
completed:
---

# 0811 — Title screen art

## Context

The title screen (`crates/ui/src/screens/title.rs`) is plain text: the words
`tactical-rpg` and "an ASCII tactics game" above the menu. It is the first
thing every player sees, and store pages and itch.io screenshots (09xx) will
show it. 0012 decides the game's name; 0801 builds the real title menu. No
ticket covered the title's look. Found while listing Chapter 1 gaps
(2026-09-29).

Look and feel rules: `docs/design/look-and-feel.md` (0011), ADR-0018 (palette
names only, glyph and half-block techniques). Nick disliked Claude-drawn
*portraits* (0021), so offer lettering and scene options, not only drawn
figures, and expect several rounds.

**Added 2026-10-01:** Nick also wants an intro cinematic on the title
(ticket 0036, built by 0817–0820). It is as long as the title song and ends
on "our logo whatever it might be", held during the song's quiet pause
(about 17 seconds). So the art chosen here is also the cinematic's last
shot: it must look right alone on the screen, with no menu under it. 0036
did not wait for this ticket (it used a stand-in logo). It is decided
(2026-10-03, *Intro cinematic* in `docs/design/title-screen.md`): the logo
and the menu show together after the first press and during the song's
silence; after 15 idle seconds the menu fades and the cinematic takes the
screen; a press brings logo and menu back. So the logo is always seen
**with** the menu under it, and the cinematic covers the whole screen in
between. When
offering option C below, tell Nick the cinematic already covers moving
pictures on the title.

## Nick input

**Decision** with the `ask-nick` and `ascii-art` skills. Render real mockups
at the game's size in palette D, for example:

- **A. Big lettered title** drawn from glyphs or half-blocks, centred, with
  the menu below.
- **B. Title plus a small scene** (a castle on a hill, a banner, crossed
  swords) drawn in glyphs.
- **C. Title over a slowly animated background** (drifting embers, rain),
  a small effect drawn by the `ui` crate.
- **D.** "Describe your own."

Sub-questions: the tagline under the title (if any), and whether the title
music (0807) starts at once or after a short fade in.

## Scope

**In:**
- The chosen title art as data (a text file under `assets/`, loaded through
  `trpg_content`, validated like other art) and drawn by `TitleScreen`.
- Replace `TITLE` / `SUBTITLE` in `title.rs` with the art and the name from
  0012.
- The window title (`crates/app/src/main.rs`, `window_title`) and the web
  page `<title>` use the name from 0012.
- Record the choice in `look-and-feel.md`.
- The layout must leave room for the web build's `Press any key` line
  where the menu goes (ticket 0034, `docs/design/title-screen.md`); show
  it in the mockups too.
- Show one mockup of the art alone (no menu, no prompt): the cinematic's
  logo shot draws it that way for a moment while the menu fades in or out
  (0036). Keep the drawing in one function that 0817 can call.

**Out (do not do):**
- The exe icon (0902).
- Store page art (09xx).

## Implementation steps

1. Mockup rounds with Nick until he picks one.
2. Store the art under `assets/` (format documented in that folder's README),
   load it in `trpg_content`, and add it to content validation.
3. Draw it in `TitleScreen`; keep the menu and help line working at every
   window size the game supports.

## Acceptance criteria

- [ ] Snapshot of the new title screen.
- [ ] Content validation loads the title art.
- [ ] Window title and web page title show the game's name.
- [ ] Nick signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: title art loader/validator.
- Snapshot / integration: title screen snapshot; existing title Harness tests
  still pass.

## Completion notes

*(Filled in by the session that completes the ticket.)*

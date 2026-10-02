---
id: "0808"
title: "Credits screen listing every third-party work"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: done
blocked_by: ["0214", "0801"]
nick_input: sign-off
completed: 2026-10-01
---

# 0808 — Credits screen

## Context

Nick (ticket 0020, [`docs/design/audio.md`](../../docs/design/audio.md) rule
3): "For any work we use, even if we can use for free, I still want to credit
so we can have a nice credits screen." This is stricter than ADR-0013, which
only required a credits screen for CC-BY. So **every** third-party work gets
credited in the game, CC0 included:
- each music track and sound (the `credits` in `assets/audio/audio.ron`, from
  0214);
- the Terminus font;
- anything else in `THIRD_PARTY_ASSETS.md`.

## Nick input

**Sign-off:** open the credits from the title screen and say whether the
layout, order and wording feel right. The layout below is a starting point,
not a decision.

## Scope

**In:**
- `assets/data/credits.ron` for the non-audio works (the Terminus font, plus
  any other `THIRD_PARTY_ASSETS.md` item that should be shown). Audio credits
  come from the audio manifest, so they're never typed twice.
- A test that every `THIRD_PARTY_ASSETS.md` row has a credit entry,
  whichever of the two files holds it, so the screen can't fall out of date.
  Vendored JS loaders that the player never sees may be marked `hidden: true`
  with a comment; they're still in `THIRD_PARTY_LICENSES.html`.
- A **Credits** entry in the title screen menu. It opens a scrolling screen
  grouped as Music, Sound effects and Fonts. Each entry reads: title, author,
  license, and the source URL shown as text. CC-BY 4.0 requires the title,
  author, license and a link, so all four are always shown.
- Cancel returns to the title.

**Out (do not do):**
- Credits for our own team or roles (Nick hasn't asked).
- Rolling end credits after the final battle.

## Implementation steps

1. Add the `credits.ron` schema, loader and validator in `trpg-content`.
   Merge it with the audio manifest's credits into one list.
2. Write the `THIRD_PARTY_ASSETS.md` coverage test (parse the table's Item
   column or match by source URL).
3. `CreditsScreen` in `crates/ui/src/screens/`, a scrolling list in the
   existing panel style (ADR-0018). Up/Down scroll with key repeat; long
   lines wrap.
4. Add the title menu entry.

## Acceptance criteria

- [x] Snapshot: the first page of the credits screen, and one scrolled page.
- [x] Every work in `THIRD_PARTY_ASSETS.md` and every audio credit appears
      (test).
- [x] Harness: Title → Credits → Cancel returns to the title.
- [ ] Nick signed off. *(Pending: open Credits from the title on the Pages
      build after the merge.)*
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: credits loader and coverage test.
- Snapshot / integration: screen snapshots, Harness navigation.

## Completion notes

**Done.**
- `assets/data/credits.ron` and `trpg_content::credits`: the file's schema,
  loader and validator, and `merge`, which joins it with the audio
  manifest's credits into `Content::credits`. Audio credits are taken from
  the cues that name them (a music cue's work goes under Music, a sound
  cue's under Sound effects), so nothing is typed twice and work we made
  ourselves isn't listed.
- Coverage test `every_third_party_asset_has_a_credit`: every row of
  `THIRD_PARTY_ASSETS.md` has a credit with the same source link, and every
  credit has a row. `THIRD_PARTY_ASSETS.md` now says so.
- `CreditsScreen` (`crates/ui/src/screens/credits.rs`) and a **Credits**
  item on the title menu, between New Game (or Quick Battle) and Quit.
- Tests: loader and merge unit tests, screen unit tests, and
  `crates/ui/tests/credits.rs` (two snapshots, Title → Credits → Cancel,
  both ends of the list, every credit reachable, every glyph in the font,
  sounds).

**What the screen looks like** (the first page; Nick's sign-off is on
this):

```
  ┌─ Credits ───────────────────────────────────────────────┐
  │                                                         │
  │ Music                                                   │
  │                                                         │
  │   "Aria" by Kistol                                      │
  │     CC0-1.0 · https://opengameart.org/content/aria      │
  │                                                         │
  │   "Battle" by mla                                       │
  │     CC-BY-4.0 · https://opengameart.org/content/battle-0│
  │                                                       ▼ │
  └─────────────────────────────────────────────────────────┘
                arrows scroll · f pause · d back
```

**Nick's changes on the PR (2026-10-02).**

> credits should auto scroll, or transition between pages. manual
> paging/scrolling can be toggled but otherwise transition every 1 or 2s.
>
> credits screen should play title screen music

- **The list rolls by itself.** Of the two (rolling or flipping pages) the
  screen rolls: a page holds eight works, too many to read in the 1 to 2
  seconds a page flip would give them, while rolling gives each work about
  a second and a half.
- **Rolling can be switched off and on:** Confirm stops it (the help line
  then offers `auto-scroll`) and Confirm starts it again.
- **The title music plays.** The screen asks for it itself, so it would
  play even if the credits were one day opened from somewhere else. Coming
  from the title it simply carries on without restarting.

*Claude's starting rules* for the parts Nick's note left open:

- The list waits 2 seconds at the top, then moves up one line every half
  second. The whole list takes about a minute.
- At the bottom it waits 2 seconds and starts again from the top.
- Pressing up or down also stops the rolling, so the player isn't fighting
  it; Confirm starts it again from where they are.
- Rolling makes no sound.

**Claude's starting choices (Nick can veto at sign-off).** The ticket
called its layout "a starting point", so these are how the screen looks
and behaves today, not decisions:

- **Where it is on the title menu:** Credits sits just above Quit.
- **Order:** Music, then Sound effects, then Fonts. Inside Music and Sound
  effects the works are in alphabetical order by title.
- **Wording of an entry:** `"Title" by Author` on one line, then the
  license and the link, dimmer, on the next. Licenses are shown by their
  short standard names (`CC0-1.0`, `CC-BY-4.0`, `OFL-1.1`).
- **Scrolling by hand:** up or down stops the rolling and moves one line
  at a time (repeating when held), with the menu tick. A small ▲ or ▼ at
  the right shows there is more above or below.
- **Leaving:** Cancel goes back to the title with Credits still
  highlighted.
- **Not shown:** the four pieces of behind-the-scenes software in
  `THIRD_PARTY_ASSETS.md` (the three web-loader scripts and the controller
  button table). The ticket allowed hiding the web loaders; the controller
  table is the same kind of thing, so it is hidden too. All four are in
  `credits.ron` under a "Software" group with `hidden: true`; removing
  that word from an entry puts it on the screen under a **Software**
  heading, if Nick would rather credit them in the game as well.

No gameplay rules were decided.

**Deviations from the plan.** None in scope. Five existing tests assumed
Quit was the title menu's second or third item and now press Down once
more (`title.rs`, `layout_picker.rs`, `controller.rs`, `game.rs`).

**For later (not done here).**
- CC BY also asks to say when a work was changed. Our music and sounds are
  converted and loudness-matched, and one sound is trimmed; the manifest's
  `note` records this but the screen doesn't show it. The ticket listed
  the four parts to show, so the screen shows those four.
- ADR-0027 says the release packages carry `THIRD_PARTY_ASSETS.md` and
  the license texts "until the credits screen exists". They still do;
  nothing was removed.
- No follow-up tickets were created.

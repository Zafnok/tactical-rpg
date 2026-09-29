---
id: "0808"
title: "Credits screen listing every third-party work"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0214", "0801"]
nick_input: sign-off
completed:
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

- [ ] Snapshot: the first page of the credits screen, and one scrolled page.
- [ ] Every work in `THIRD_PARTY_ASSETS.md` and every audio credit appears
      (test).
- [ ] Harness: Title → Credits → Cancel returns to the title.
- [ ] Nick signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: credits loader and coverage test.
- Snapshot / integration: screen snapshots, Harness navigation.

## Completion notes

*(Filled in by the session that completes the ticket.)*

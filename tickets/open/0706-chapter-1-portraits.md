---
id: "0706"
title: Chapter 1 cast portraits (bought art, all expressions)
type: content
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0701", "0703", "0011", "0020", "0110", "0710"]
nick_input: sign-off
completed:
---

# 0706 — Chapter 1 cast portraits

## Context

Portraits for every character who speaks in Chapter 1, using the portrait
briefs in `docs/story/characters/*.md` (0701).

**Changed 2026-09-28:** Nick doesn't want Claude-drawn portraits. He chose
bought art, CaptainSkolot's portrait bundle (0020 records the decision and
its rules). This ticket now **assigns bought portraits** to the cast
instead of drawing them. The files come from the private assets repo (0110)
and use the 64×64 PNG format (0710).

## Nick input

**Sign-off:** for each Chapter 1 speaker, Claude proposes two or three
candidate faces from the bought packs (rendered in the dialogue screen,
neutral plus one other expression), and Nick picks one or asks for others.
Characters no pack fits follow Nick's answer to 0020 question 3.

## Scope

**In:** one portrait per Chapter 1 speaking character (the player-gendered
lead gets **two**: `lead_m` and `lead_f`, per `setting-and-tone.md`), with
the five required expressions mapped to the pack's expressions, plus any
extra ones listed in the character sheet. One shared `soldier` portrait for
unnamed enemies.

**Out:** later chapters; drawing new art (unless 0020 allows edits, and then
only the edits it allows).

## Implementation steps

1. List Chapter 1 speakers from `docs/story/chapters/ch01.md`.
2. For each, read the portrait brief and shortlist two or three bought
   faces that match it (age, build, class, colours).
3. Render the candidates in the dialogue screen (a rendered PNG, as in 0704)
   and send them to Nick. Record his picks.
4. Import each pick with `cargo xtask portrait-import` (0710) into
   `assets-private/portraits/`. Map `neutral`, `happy`, `angry`, `sad` and
   `surprised` to the closest pack expressions, and note the mapping in the
   character sheet.
5. Update `docs/story/characters/*.md` with which pack and face each
   character uses. List each pack in `THIRD_PARTY_ASSETS.md` (marked private).

## Acceptance criteria

- [ ] Every Chapter 1 speaker has a validated portrait with all required
      expressions.
- [ ] Characters are distinguishable in a greyscale screenshot.
- [ ] Nick approved each pick.
- [ ] All gates in the `run-gates` skill pass.

## Completion notes


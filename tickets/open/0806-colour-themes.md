---
id: "0806"
title: Colour themes option (C, D, E, G palettes)
type: feature
milestone: M7 Chapter 1
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0805"]
nick_input: sign-off
completed:
---

# 0806 — Colour themes option

## Context

In ticket 0011 Nick chose palette **D "Earthy painterly"** as the game's look,
and asked for four palettes to be offered as a player setting: **C Rich &
painterly, D Earthy painterly (default), E War-table parchment, G Moonlit**
(`docs/design/look-and-feel.md`). Their mockup values are in that file, and
screenshots are in `docs/screenshots/0011-theme-{c,d,e,g}.png`. Code refers to
colours only by palette name (ADR-0018), so a theme is a data swap. Options
screen: 0805.

## Nick input

**Sign-off:** Nick switches between the four themes in the options screen and
says whether each still looks like its mockup; tweaks become `tuning` tickets.

## Scope

**In:**
- Three more palette files, e.g. `assets/data/palettes/{painterly,parchment,moonlit}.ron`,
  each with every name in `palette.ron`. D stays `assets/data/palette.ron` (or
  move all four into `palettes/` and update `PALETTE_PATH`: pick one and keep a
  single code path).
- A `theme` setting (`Settings`, 0805), default D, saved like the others.
- Loading the chosen palette at startup and on change (rebuild `Ctx.palette`).

**Out (do not do):**
- Colour-blind palettes (a later ticket).
- Portrait colour variants per theme: portraits keep their own colours.

## Implementation steps

1. Fill each theme file from the tables in `look-and-feel.md`. Derive the names
   the tables don't list (`ally`, `neutral`, `heal_range`, `danger_zone`,
   `exp_bar`, `stone*`, `road*`, `path`, `hp_mid`, panel colours) to match that
   theme's feel. E is **light**: keep `black` as its light background only if
   `black` means "console background" everywhere; otherwise add a
   `background` name and switch the renderer/screens to it in this ticket.
2. Content validation: every theme file has exactly the same names as D.
3. `Settings.theme: Theme` (`Painterly | Earthy | Parchment | Moonlit`), an
   options row with Left/Right to change it, applied immediately.
4. Check the dimming code (acted units, listener portraits) lerps toward the
   background colour so it works on the light theme (ADR-0018).

## Acceptance criteria

- [ ] All four themes selectable; the choice persists across restart.
- [ ] Content test: every theme has the same colour names as D.
- [ ] Snapshots (colour names) of the Quick Battle screen in each theme.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: theme file validation; settings round-trip.
- Snapshot / integration: Quick Battle in each theme; Harness test that
  changing the option changes `ctx.palette`.

## Completion notes


---
id: "0429"
title: Show what a skill or art does while choosing it in battle
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 0429 — Show what a skill or art does while choosing it

## Context

Nick: "we need this info readily available when selecting in battle. It's a
nuisance to have to know each skill by name."

Today a skill's effect is written out in one place only: the unit info
screen's *Skills* block (0412), one line per skill from
`effect_text(&SkillDef)` in `crates/ui/src/screens/battle/skills.rs`. The two
places where the player actually *chooses* one show only names:

- **Skill menu** (action menu → `Skill`, 0412): `skill_menu` in
  `skills.rs` draws `Brace  3 dur  Wpn 20/20`. No effect.
- **Arts list** while targeting (0414): `art_menu` in
  `crates/ui/src/screens/battle/art_list.rs` draws
  `Guard Break     −4 dur  D`. No effect. The forecast shows the chosen
  line's non-number notes (`pierces`, `pins: Mov −3`, from `note_text`), but
  not its hit/crit/damage changes or what it's for in words.

The spell menu (0410, not built yet) has the same gap in its plan; this
ticket also adds the requirement there (see *Scope*).

Relevant: `docs/design/look-and-feel.md` (layout and colours are Nick's),
`docs/design/combat-arts.md`, `docs/design/progression.md` (*Skills*),
ADR-0004 (only `ui`/`app` draw; text comes from `core` data).

## Nick input

**Layout first (look & feel is Nick's call, never pick it yourself):** before
coding, use the `ascii-art` skill to render at least three real mockups of
the Skill menu and the arts list with a description shown for the
highlighted line, and ask Nick with the `ask-nick` skill. Example options
(drawn from real games; add others if they fit):

1. **Description line under the list** (Fire Emblem: Three Houses' arts
   list: a text strip for the focused art).
2. **Side box beside the list** (Fire Emblem Engage / Fates skill
   window: name, cost and effect in a small panel next to the list).
3. **Effect in the help bar** at the bottom of the screen (Advance Wars /
   Into the Breach-style one-liner for the hovered item).
4. Describe your own.

Record the answer in `look-and-feel.md` (a *Descriptions in lists* rule, worded
as a player situation with an example, per Nick's preference) so later menus
(0410 spells, 0408 preparations, 0603 promotion) follow it.

**Sign-off:** in Quick Battle, open the lord's and the knight's `Skill` menu
and the archer's and lord's arts list; move through the lines. Can you tell
what each one does without leaving the menu?

## Scope

**In:**
- A description for the **focused** line of the Skill menu (skill effect).
- A description for the **focused** line of the arts list: `Attack`
  (plain attack), each Combat Art, and each combat active.
- An art effect text function (arts have none yet), same style as
  `effect_text` for skills.
- The description updates as the focus moves; dimmed lines still show
  theirs (so the player learns what they can't afford).
- The `look-and-feel.md` rule from Nick's answer.
- One line added to 0410's *Scope → In* and step 1: the spell list shows the
  focused spell's description per the `look-and-feel.md` rule. **If 0410 is
  already in `tickets/done/`** (the two tickets don't block each other),
  write a follow-up ticket instead (`write-ticket`) that adds the
  description to the spell list.

**Out (do not do):**
- Rewording the info screen's Skills block (it already has effect lines),
  unless Nick's chosen layout asks for the same text there.
- Flavour/lore text or hand-written descriptions per skill in data: build
  the text from the effect data (so it can't drift from the rules). If Nick
  wants written descriptions, write a follow-up ticket.
- Item/consumable menus (0407 already shows `effect_text` for consumables).
- Spell menu code (0410).
- Any rule or number change.

## Implementation steps

1. Ask Nick for the layout (see *Nick input*); record it in
   `docs/design/look-and-feel.md`. Do not start step 3 before the answer.
2. **Art text:** in `art_list.rs`, add `pub fn art_effect_text(art: &ArtDef)
   -> String` built from `ArtEffect` (read `crates/core/src/art.rs` and the
   field list at the top of `assets/data/arts.ron`): number changes first
   (`+20 hit`, `Mt +3`, `eff. vs Armored ×3`, `min range 2`, sword follow-up,
   axe min damage), then the existing `note_text` notes, joined with `; `.
   Reuse `mods_text`/`timed_text`/`stats_text` from `skills.rs` where they
   fit. Add `Technique::effect_text(&self, state)` returning the art text, the
   skill's `effect_text`, or `plain attack` for `Attack` (check the exact
   variant names in the `Technique` enum at the top of `art_list.rs`).
3. **Draw it** where Nick chose, for the focused line of:
   - the arts list (`draw_list` in `art_list.rs`; the focus is the `Menu`'s
     focused index),
   - the Skill menu (the call site is in `mode.rs` near
     `skill_menu(state, sel.unit, &choices)`; `SkillChoice` holds the skill).
   Text colour: `UiColor::TextDim` like the info screen's effect line; cut
   long text with `pen.cut`/the existing truncation helper rather than
   overflowing the box. Must fit the right-handed and left-handed layouts
   (0208) and not cover the attacker or the target (see `list_origin`).
4. If the chosen layout puts text in the help bar, follow the
   `keyboard-input` skill (key names from the keymap, never literals).
5. Edit `tickets/open/0410-*.md` as described in *Scope*, or write the
   follow-up ticket if 0410 is already done.

## Acceptance criteria

- [ ] `look-and-feel.md` has the *Descriptions in lists* rule with Nick's
      choice and an example.
- [ ] Unit test `art_effect_text` for every art in `arts.ron` (each returns a
      non-empty string; spot-check at least three exact strings, e.g. an art
      with `no_counter`, one with `on_first_hit`, one with `effective`).
- [ ] Harness: in the arts list, moving Down changes the description to the
      next line's text; same for the Skill menu (Brace → `self: Def +5 Res
      +5`).
- [ ] Harness: a dimmed line still shows its description when focused
      (if the list lets focus reach it; otherwise document why not).
- [ ] Snapshots: arts list with description (archer and lord), Skill menu
      with description, both in the left-handed layout too.
- [ ] 0410 mentions the description in its scope and step 1 (or, if 0410
      was already done, a follow-up ticket for the spell list exists).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `art_effect_text` strings; `Technique` effect text for `Attack`,
  an art and an active.
- Snapshot / integration: harness focus changes and the snapshots above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*

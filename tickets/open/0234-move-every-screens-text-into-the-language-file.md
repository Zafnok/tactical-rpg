---
id: "0234"
title: Move every screen's text into the language file
type: feature
milestone: M1 Engine
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0233"]
nick_input: none
completed:
---

# 0234 — Move every screen's text into the language file

## Context

0233 built screen text by key ([ADR-0045](../../docs/adr/0045-languages-text-by-key-and-line-ids.md))
and converted the title screen. About 200 player-facing literals remain in
`crates/ui/src`. This ticket moves them, following the title screen's
pattern, so a language pack can cover the whole game.

## Nick input

None. Nothing a player sees changes.

## Scope

**In:** every literal `cargo xtask check-text` lists, in every screen and
widget under `crates/ui/src` (battle, dialogue, results, save, class
change, key bindings, layout picker, lead select, mode select, game over,
credits, the flow screen, help bars, banners such as `PLAYER PHASE`).

**Out (do not do):** the debug menu and debug tools (English only, for
developers); names and descriptions from data files, tips and dialogue
(0235); rewording anything; layout changes. If moving a literal shows a
layout that only works for that English word's length, leave the layout
and note it for 0237.

If this is too big for one PR (over ~600 changed lines besides
`ui.ron`), split by screen group into two PRs of this same ticket's
pattern: write the second as a new ticket and lower the `check-text`
number in each.

## Implementation steps

1. Run `cargo xtask check-text` for the list.
2. Per screen: replace each literal with `ctx.text("<screen>.<thing>")`
   or `ctx.text_with(...)` for text with numbers or names in it, and add
   the key to `assets/lang/en/ui.ron`, grouped by screen with a comment
   line per group. One key per meaning: don't share a key between two
   places just because the English matches (`Cancel` the button and
   `Cancel` the action name may differ in Japanese).
3. Text built by joining pieces (`"Lv " + n`) becomes one key with a
   placeholder (`"Lv {level}"`), because word order differs by language.
4. Screens where new screens were added since 0233 (check `git log` for
   `crates/ui/src/screens`) are covered too.
5. Set the number in `check-text` to 0. What it lists that is not text
   for a translator (a picture drawn from rows of `X` and `.`, a
   controller button's name) gets a `// check-text: not player text`
   comment instead of a key.
6. Add a line to `crates/ui/src/screens/README` or the `work-ticket`
   skill's checklist, wherever new-screen rules live: new screens take
   their text from `ctx.text`.

## Acceptance criteria

- [ ] `cargo xtask check-text` reports 0 and its limit is 0.
- [ ] No snapshot changes.
- [ ] `cargo xtask lang-status test` still works (the test pack stays partial on purpose).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot / integration: existing snapshots unchanged is the test.

## Completion notes


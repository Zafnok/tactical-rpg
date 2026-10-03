---
id: "0825"
title: "Options: Language, and however the first launch picks one"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0042", "0233", "0805"]
nick_input: answer-first
completed:
---

# 0825 — Options: Language, and however the first launch picks one

## Context

0233 put a language in `Ctx`; nothing lets a player change it. Nick
decides in 0042 how a language is picked (asked on first launch, follows
the system, or Options only) and how a machine translation is labelled
(`docs/design/voices-languages-and-script.md`, *Japanese*). 0805 builds
the Options screen and `Settings`.

## Nick input

**Answer first:** ticket 0042 (questions 2 and 3). Build exactly what the
design doc says; if it doesn't cover something a player would see, stop
and ask through `ask-nick`.

## Scope

**In:** `Settings::language`; an Options row; the first-launch behaviour
0042 chose; the machine-translation label 0042 chose.

**Out (do not do):** the translation (0719); a kana name-entry screen (its
own ticket if 0042 asks for one); store page text (0907).

## Implementation steps

1. `Settings` (0805) gains `language: LangCode`, default `en`; bump the
   settings version and migrate old settings to `en`. Loaded into
   `Ctx::lang` at start-up.
2. Options screen: a "Language" row listing English and every pack in
   `Content::lang` except `test` (which shows only with
   `Ctx::debug_tools`). Each language is shown by its own name from
   `lang.ron` (`日本語`), whatever the current language is. Changing it
   takes effect at once on every screen.
3. The label for `made_by: Machine` packs, as 0042 decided (beside the
   name, a one-time notice, or nothing in game).
4. First launch, as 0042 decided. If "follows the system": `app` reads
   the OS / browser language once and passes it to `Ctx` (only `app` may
   ask the platform, ADR-0004); a system language with no pack gives
   English.
5. A pack that names a language whose glyphs the atlas lacks can't
   happen (0236's atlas test), but until 0236 lands the row hides any
   pack whose own name can't be drawn.

## Acceptance criteria

- [ ] Choosing a language in Options changes the title screen's text at once and survives a restart (Harness test with the `test` pack).
- [ ] Old saved settings load with English (test).
- [ ] The first-launch behaviour matches the design doc (Harness test).
- [ ] The machine-translation label appears as decided (snapshot).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: settings migration.
- Snapshot / integration: the Options row; switching; first launch.

## Completion notes


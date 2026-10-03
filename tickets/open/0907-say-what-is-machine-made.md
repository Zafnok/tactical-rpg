---
id: "0907"
title: "Say what is machine-made: credits, itch page, Steam's AI survey"
type: feature
milestone: M8 Release
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0042", "0043"]
nick_input: sign-off
completed:
---

# 0907 — Say what is machine-made: credits, itch page, Steam's AI survey

## Context

By Nick's direction (`docs/design/voices-languages-and-script.md`) the
game may ship AI-generated voices, a machine-translated Japanese option,
and a script written by Claude, each to be replaced by paid people later.
Steam requires pre-generated AI content to be described in its content
survey and shows that text on the store page; Japanese players judge
unlabelled machine translation harshly; and Nick credits everyone
(`audio.md` rule 3). ADR-0045 and ADR-0046 record who made each pack and
clip (`made_by`), so the credits can be generated, and change by
themselves when a human replaces something.

## Nick input

**Sign-off** on the wording: the credits lines and the store-page
paragraph are shown to him in the PR description. 0042 (question 3) and
0043 (question 6) already fixed the in-game labels; this ticket doesn't
reopen them.

## Scope

**In:** credits screen sections built from data; a store-text file; a
line in 0901 and 0903.

**Out (do not do):** the in-game labels in Options (0825, 0826);
publishing anything (0901, 0903 do that).

Either order with the features: build each section so it shows only when
the build has that thing (a voice manifest with `Generated` clips; a
`Machine` pack). If voices or Japanese haven't landed, their section is
simply absent.

## Implementation steps

1. Credits screen (`crates/ui/src/screens/credits.rs`,
   `assets/data/credits.ron`): add sections
   - **Voices:** per cast entry, the character and either "AI-generated
     voice (<tool>)" or the actor's name, from the voice manifest's
     `made_by`.
   - **Translation:** per language pack, "Machine translation" or the
     translator's credit, from `lang.ron`.
   - **Story and script:** the wording Nick signs off (who directed the
     story; that the script was written with an AI model). Put the text
     in `credits.ron`, so a hired writer's name can be added by a data
     edit.
2. `docs/release/store-disclosure.md`: the paragraph for Steam's "AI
   generated content" survey question (pre-generated: voices,
   translation, script; no live generation), and the matching short
   paragraph for the itch page, each listing only what the release
   actually contains. Include the Japanese store text's note that the
   translation is machine-made.
3. 0901 and 0903 already say to use this file (their *Machine-made
   content* paragraphs). If either is done by now, its page needs the
   text: write the exact paste-in steps for Nick in the PR description.
   The file also lists which of Steam's language boxes (Interface /
   Subtitles / Full Audio) to tick for what ships.
4. Text through `ctx.text` (0233) where it has landed.

## Acceptance criteria

- [ ] With a test manifest holding one `Generated` and one `Recorded` clip, the credits show both correctly (Harness test).
- [ ] With a `Machine` pack the credits say so; with `Human("Name")` they show the name (test).
- [ ] With neither voices nor packs the credits are unchanged (existing snapshot).
- [ ] `store-disclosure.md` exists and 0901 / 0903 point at it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot / integration: the credits sections.

## Completion notes


---
id: "0708"
title: "Dialogue: lead reply choices and lead name/pronoun tokens"
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0702", "0704"]
nick_input: none
completed:
---

# 0708 — Dialogue: lead reply choices and lead name/pronoun tokens

## Context

Ticket 0007 decided the lead is a **Persona-style lead the player shapes**
([`docs/design/setting-and-tone.md`](../../docs/design/setting-and-tone.md),
section "The lead"):

- At key moments the player picks one of 2–3 reply tones. Other characters
  react differently in a line or two, then the scene **rejoins**. Choices never
  branch the plot.
- The player picks the lead's gender at New Game, so the script is
  gender-neutral and refers to the lead by name or pronoun tokens.

The `.dlg` format and `DialoguePlayer` (0702) and the `DialogueScreen` (0704)
have neither feature. This ticket adds both. The New Game gender picker lives
in 0801, which uses the `LeadProfile` type defined here.

## Nick input

None.

## Scope

**In:**
- `@choice` blocks in `.dlg` (format spec, parser, validator).
- Lead tokens in text: `{lead}` (name), `{they}`, `{them}`, `{their}`,
  `{theirs}`, `{themself}`, with capitalised forms `{They}` etc.
- `core::lead::LeadProfile { name: String, gender: LeadGender }` with
  `LeadGender { Male, Female }` and a `pronouns()` table (serde, pure).
- `DialoguePlayer`: choice state + `choose(i)`; token substitution given a
  `&LeadProfile`.
- `DialogueScreen`: draw the choice list and select with `j/k` + confirm.
- The lead's portrait is chosen by gender: the character id `lead` resolves to
  portrait `lead_m` or `lead_f`. Add a `lead` character entry and two
  placeholder portraits (`lead_m`, `lead_f`, clearly marked as placeholders,
  per the `ascii-art` skill) so tests and 0801 have something to show.

**Out (do not do):**
- The New Game gender/name screen and storing `LeadProfile` in `Campaign` (0801).
- Remembering which reply was picked, or any effect of choices beyond the
  reaction lines (nothing to record: choices don't branch, per the design).
- Real lead portraits (0706; they replace the placeholders) and real scenes (0707).

## Implementation steps

1. **Format** (extend `assets/dialogue/README.md` with examples):
   ```
   @choice
   * earnest: We do this properly, or not at all.
     bors[surprised]: ...Huh. Fine.
   * wry: I've had worse mornings. Not many.
     bors[happy]: Ha! There's the spirit.
   * blunt: Stop talking. Move.
     bors[angry]: Charming as ever.
   @endchoice
   ```
   `* <tone>: <text>` is an option (the text is what the lead says and is shown
   in the menu). The lines indented under it are that option's reaction steps.
   Any normal step (`Say`, `Narrate`, `@left`…) is allowed there, but no nested
   `@choice`. After `@endchoice`, every option rejoins the scene.
2. **Parser:** new `Step::Choice { options: Vec<ChoiceOption { tone, text, steps: Vec<Step> }> }`.
3. **Validator** (file:line errors, each tested with its exact message):
   fewer than 2 or more than 3 options; missing `@endchoice`; nested `@choice`;
   option text > 60 chars (it must fit the menu); an option's reaction has more
   than 4 text steps (keep rejoins tight); a reaction leaves a different
   character on screen than the other options do at `@endchoice` (screen state
   must be identical after rejoin); unknown token `{...}`; a `lead:` line
   longer than 40 chars (see step 4).
4. **Short neutral lead lines:** outside choices the lead speaks only in short
   neutral lines (design rule 1). Allow `lead: <text>` when the text is ≤ 40
   chars, so "Let's move." works but monologues don't. Longer lines are a
   validator error whose message points to `setting-and-tone.md`.
5. **Tokens:** `LeadProfile::pronouns()` returns they/them/their/theirs/themself
   equivalents (he/him/his/his/himself, she/her/her/hers/herself). Substitution
   runs in `DialoguePlayer::current()` so the stored scene is unchanged. The
   validator checks the text-length limit against the longest possible
   substitution (name max length: 12 chars, a const).
6. **`DialoguePlayer`:** `View` gains `choices: Option<Vec<&str>>`. When the next
   step is a `Choice`, `current()` shows the options and `advance()` does
   nothing until `choose(i)` runs, which plays that option's steps and then
   continues after the block.
7. **`DialogueScreen`:** when `choices` is `Some`, draw a list above the text
   box (highlighted row, `j/k` or arrows to move, confirm to pick). Skip or
   fast-forward stops at a choice. Snapshot both variants.
8. Update `assets/dialogue/test.dlg` with one choice block and tokens. Update the
   `story-writing` skill's format section with the rules from
   `setting-and-tone.md` (few lead lines, choice budget, gender-neutral text).

## Acceptance criteria

- [ ] Spec documents `@choice` and tokens with a full example.
- [ ] Every new validator error has a test with the exact message.
- [ ] `DialoguePlayer` on the test scene: each option leads to the same `View` after `@endchoice` (test).
- [ ] Token substitution is correct for both genders and a custom name (tests).
- [ ] Snapshot of the choice menu, and the `lead` portrait resolves to `lead_m`/`lead_f`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parser for choice blocks; validator errors; pronoun table; substitution.
- Property: for random scenes with choice blocks, every option path visits the post-choice steps exactly once, in order.
- Snapshot: `DialogueScreen` with a 2-option and a 3-option choice.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*

---
id: "0708"
title: "Dialogue: lead reply choices and lead name/pronoun tokens"
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: done
blocked_by: ["0702", "0704"]
nick_input: none
completed: 2026-09-30
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

- [x] Spec documents `@choice` and tokens with a full example.
- [x] Every new validator error has a test with the exact message.
- [x] `DialoguePlayer` on the test scene: each option leads to the same `View` after `@endchoice` (test).
- [x] Token substitution is correct for both genders and a custom name (tests).
- [x] Snapshot of the choice menu, and the `lead` portrait resolves to `lead_m`/`lead_f`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parser for choice blocks; validator errors; pronoun table; substitution.
- Property: for random scenes with choice blocks, every option path visits the post-choice steps exactly once, in order.
- Snapshot: `DialogueScreen` with a 2-option and a 3-option choice.

## Completion notes

**Done (2026-09-30).**

- `trpg_core::lead`: `LeadProfile { name, gender }`, `LeadGender { Male, Female }`,
  the pronoun table, `portrait_id()` / `portrait_for()` (`lead` → `lead_m` /
  `lead_f`), token substitution, and `longest_len()` for the validator
  (`MAX_NAME_LEN = 12`, `DEFAULT_NAME = "Ellery"`). Serde round-trip tested.
- `.dlg`: `@choice` / `* tone: text` / two-space-indented reactions /
  `@endchoice`, parsed into `Step::Choice { options: Vec<ChoiceOption> }`;
  `print_scene` writes them back (round-trip property test includes choices).
- Validator (each error tested with its exact message): 2–3 options, missing
  `@endchoice`, nested `@choice`, reply text > 60, reaction > 4 text lines,
  different characters (or caption) on screen after a reaction, unknown
  `{token}` and unclosed `{`, `lead:` line > 40 (message points to
  `setting-and-tone.md`), plus syntax errors (option outside a choice, bad
  option line, wrong reaction indent…). Lengths count tokens at their longest.
  The lead's expressions are checked against both lead portraits.
- `DialoguePlayer`: takes a `LeadProfile`; `View` gains `choices` (text,
  caption and replies are `Cow<str>` with tokens filled in; the stored scene is
  unchanged); `choose(i)`, `is_choosing()`, `skip_to_choice()`.
- `DialogueScreen`: the replies are listed in the text box under the line
  being answered, and the box grows upward to fit (Nick's pick B; the `Menu`
  widget handles focus and sounds), picked with the layout's cursor up/down and Confirm (or End
  turn); skipping stops at a choice. The lead's name plate shows the
  player's name and the gendered portrait. `Ctx.lead` holds a placeholder
  profile until 0801.
- `lead` character in `characters.ron` (placeholder stats copied from
  `test_lord`), placeholder portraits `lead_m` / `lead_f` (recoloured
  `test_lord`), `test.dlg` gained a lead, a 3-reply choice and tokens.
- Docs: `assets/dialogue/README.md` (full example, tokens table, rules),
  `assets/portraits/README.md`, `story-writing` skill ("Writing the lead").
- Snapshots: 3-reply choice (test scene) and 2-reply choice with a female
  lead named Isolde.

**Deviations**

- The ticket says `j/k` to move: the key layouts bind the cursor to arrows
  (right-handed) or W/S (left-handed), and game code only sees `Action`s, so
  the menu uses `CursorUp`/`CursorDown` (whatever the player bound).
- `View` holds `Cow<str>` instead of `&str` (substituted text must be owned)
  and is no longer `Copy`.
- The portrait viewer's tests now filter to the two test portraits, so they
  don't change whenever a portrait is added.
- `_typos.toml`: allowed `abd`, a run of colour keys in a snapshot grid (same
  as the existing `iy`).

**Starting rules and Nick's answers** (2026-09-30, recorded in
`docs/design/setting-and-tone.md`):

1. After a reply's reaction, the portraits stay as the reaction left them and
   the script sets any expression change (Nick: "I think the written script
   should determine reaction transitions"; replaces my first rule, which reset
   expressions automatically).
2. While the replies are up, the line being answered stays in the text box;
   the picked reply is not repeated as a text box (Nick: "sure").
3. Back does nothing while the replies are up, and "Skip scene" stops at each
   choice (Nick: "sure").
4. The replies are listed inside the text box, under the line being
   answered, like Stardew Valley; the box grows upward when they don't fit
   (Nick picked option B from three mockups).
5. The lead's default first name is **Ellery** (Nick turned down Rowan:
   "lame name, pick something better"; still renameable). Until New Game asks
   the player (0801), the lead is Ellery, male (*Claude's starting rule*).
6. Writers: `{They}` becomes He/She, so verbs agree with he/she. Claude
   writes all scripts (Nick: "I'm not writing anything, that's all you").

**Follow-ups:** none created.

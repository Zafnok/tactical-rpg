---
name: story-writing
description: Write or revise story material — story bible, character sheets, outline, chapter beat sheets, and .dlg dialogue scripts — following the pipeline in ADR-0011. Use for any 07xx story ticket or whenever dialogue/cutscene text is written.
---

# Story writing

Read `docs/adr/0011-story-authoring-pipeline.md` first. The quality comes from
the process; do not skip steps.

## Canon order (higher wins on conflict)

1. `docs/story/beats.md` — Nick's beats. Never contradict. If a beat is
   ambiguous, pick an interpretation and flag it in the PR.
2. `docs/design/*.md` — game systems (e.g. if magic exists, how it works).
3. `docs/story/bible.md`, `characters/*.md`, `outline.md`.
4. `docs/story/ledger.md` — what has happened so far.

## Names are variables

Nick may rename anything (`docs/story/names.md`). Every proper noun has a
stable id there; add new names there first. In `.dlg` scripts, speakers are
role ids and names in text are name tokens (format: ticket 0709); never write a
registered display name literally. The story docs use display names; a
rename is a find-and-replace across `docs/story/` in the same commit as the
registry change.

## Craft rules

- **Every named character** has: want (external goal), need (what they must
  learn), flaw, a secret or pressure, and an arc (start state → end state).
  Personal stories are the collision of wants between characters.
- **Every scene changes something** — information, relationship, plan, or
  stakes. If nothing changes, cut it.
- **Voice:** each character sheet has voice notes and 3 sample lines. Before
  writing a scene, reread the sheets of everyone in it. Vocabulary, sentence
  length and what they refuse to say should differ between characters.
- **Show, don't tell.** No character explains a thing both speakers already know.
- **Exposition budget:** max 4 consecutive text boxes of explanation; break it
  with reaction, conflict or a joke.
- **Tone** follows `bible.md`. Humour is allowed in dark stories; it makes the
  dark parts land.
- **Gameplay tie-in:** pre-battle scenes set up *why this fight*; in-battle
  lines react to what the player is doing (boss taunt, recruit conversation,
  death quote); post-battle scenes pay off consequences.

## Format constraints

- Dialogue goes in `assets/dialogue/*.dlg`. The format and every rule the
  validator enforces are in `assets/dialogue/README.md`; read it before
  writing a script. Run `cargo test -p trpg-content` to validate.
- Two portraits on screen max: `left` and `right`. Only use expressions listed
  in the character's sheet.
- Text box is 3 lines × ~70 characters. Keep each line ≤ 2 boxes (~200 chars).
- Plain ASCII punctuation in scripts (`'` `"` `...` `--`) — the font may not
  have smart quotes.

## Writing the lead

The lead is Persona-style (`docs/design/setting-and-tone.md`, "Rules for
writing the lead"); the validator enforces the limits below.

- **Few lines.** Outside reply choices the lead (`lead:`) speaks only in
  short, neutral lines of at most 40 characters ("Let's move."). Other
  characters carry scenes.
- **Reply choices** (`@choice` … `@endchoice`) at key moments: 2–3 replies
  with distinct tones (earnest / wry / blunt…), each at most 60 characters.
  Each reply gets a reaction of at most 4 text boxes, then the scene rejoins
  and goes on the same way for every reply. Choices never branch the plot,
  recruit or lose units, or change the ending. Budget: about 1–3 choice
  points per chapter.
- **Never fix the lead's personality** outside what the player picks: no
  cruelty, jokes or strong opinions in `lead:` lines.
- **Gender-neutral text.** The player picks the lead's gender and first name,
  so refer to the lead with `{lead}` and `{they}` `{them}` `{their}`
  `{theirs}` `{themself}` (`{They}` etc. at the start of a sentence). They
  become he/she, so the verb agrees with he/she: `{They} knows`, not
  `{They} know`. No line may depend on the lead's gender.

## Critique pass (mandatory before committing a script)

Reread the whole script as an editor, separately from writing it, and fix:

- [ ] Any line a different character could have said unchanged (voice failure).
- [ ] Exposition dumps, "as you know" dialogue.
- [ ] Contradictions with beats, bible, ledger.
- [ ] Scenes where nothing changes.
- [ ] Lines over length budget; unknown expressions/speakers.
- [ ] The chapter advances the main plot AND at least one personal arc.

Then update `docs/story/ledger.md` with what changed for each character.

## Gates

Stop and ask Nick (via the `ask-nick` style: short summary, options) only at:
bible + cast summary, and act outline. Present a **one-page summary**, not the
full documents.

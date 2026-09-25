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

- Dialogue goes in `assets/dialogue/*.dlg` (format in ticket 0702 / its docs).
- Two portraits on screen max: `left` and `right`. Only use expressions listed
  in the character's sheet.
- Text box is 3 lines × ~70 characters. Keep each line ≤ 2 boxes (~200 chars).
- Plain ASCII punctuation in scripts (`'` `"` `...` `--`) — the font may not
  have smart quotes.

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

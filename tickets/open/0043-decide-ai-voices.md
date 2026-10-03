---
id: "0043"
title: "Decide: AI voices — what is voiced, each voice by ear, on or off, the tool"
type: design-decision
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0043 — Decide: AI voices — what is voiced, each voice by ear, on or off, the tool

## Context

Nick, 2026-10-03 (`docs/design/voices-languages-and-script.md`): voice
acting by AI-generated voices for now, "with options menu to turn it off,
maybe even ask upon starting a new game", replaced by a real cast once the
game earns enough. Not a blocker for Chapter 1 or Act 1. The technical
plan is [ADR-0046](../../docs/adr/0046-voice-clips-by-line-id.md). This
ticket settles everything a player hears or sees, the way 0020 settled
music: by ear, over several rounds.

## Nick input

**Decision.** `ask-nick` skill. Steps 1–2 are Claude's research; Nick
answers from step 3. He may need to do a **setup** step if a tool needs an
account or an install on his machine: give exact clicks, and never ask him
for a key in chat.

## Scope

**In:** the tool research and its licence check; a listening page; Nick's
answers recorded in `docs/design/voices-languages-and-script.md` (a
*Voices* section) and `docs/design/README.md`; the cast list (character →
voice) as a table in that doc.

**Out (do not do):** game code; the generation tool (0721); generating a
chapter (0722). No sample that clones or imitates a real person, actor or
existing character. No commissioning anyone (Nick's rule).

## Implementation steps

1. **Research the tools.** For each candidate, read its terms on the day
   and note: who owns the output, commercial use, royalties, price, whether
   it runs on Nick's machine (Windows 11, no MSVC; say what hardware it
   needs) or as a service, Japanese support, and whether a designed voice
   stays the same across hundreds of lines. Cover both kinds:
   - free models that run locally under a licence allowing commercial use
     of the output (check the *weights'* licence, not just the code's;
     many are non-commercial);
   - paid services (monthly price for one chapter's worth and for a whole
     game's worth of lines).
   ADR-0046 rule 8 is the bar. Drop anything that fails and say why.
2. **Build a listening page** (an Artifact, like 0020's): for each tool
   that passed, the same four Chapter 1 lines (`assets/dialogue/ch01.dlg`)
   for three characters with different voices in their sheets
   (`docs/story/characters/*.md`, *Voice notes*): Hollis (old, dry), the
   lead's short lines in both genders, Harl (the boss). Keep the page's
   source in `assets-src/audio/`.
3. **Ask Nick**, one at a time:
   1. **Which tool**, by ear, with its price beside it. Paying is his call.
   2. **What is voiced.** Options: every story line (Triangle Strategy,
      Unicorn Overlord); only the big scenes, the rest silent text; a
      short reaction per text box, a grunt or a word (Fire Emblem
      Awakening); narration read or not. Give an honest size for each (how
      many lines in Chapter 1).
   3. **Battle lines.** None yet; or boss, crit and death lines only (the
      scripts already have those scenes).
   4. **The lead.** Unvoiced (Byleth in Three Houses); voiced in both
      genders, with lines that say the player's name left silent or
      reworded.
   5. **On or off at the start, and asking.** Options: a question when a
      new game starts ("Play with AI-generated voices?"); on, with the
      switch in Options; off, with the switch in Options. Note that Steam
      shows an AI notice on the store page either way.
   6. **How it is labelled** in the game: the Options row's wording, a
      line in the credits.
   7. **Text and voice together.** Options: text types out as now and the
      clip plays over it; advancing cuts the clip (most games); an
      auto-advance mode when the clip ends (a later ticket if wanted).
   8. **Japanese.** English voices under Japanese text; Japanese voices
      too (double the work, and nobody here can check them); no voices in
      Japanese.
   9. **The cast.** Per Chapter 1 speaking character, two or three voice
      candidates from the chosen tool reading that character's sample
      lines; Nick picks. Later chapters' characters get their own round
      when they are written.
4. Record answers verbatim, then as rules. Write the tool's row in
   `THIRD_PARTY_ASSETS.md` (terms quoted, date read).
5. Update 0720, 0721, 0722 and 0826 where an answer changes them, and
   write tickets for anything new (auto-advance, battle voice lines).

## Acceptance criteria

- [ ] Every question has Nick's answer, verbatim, in the design doc.
- [ ] The chosen tool's terms are quoted with date and URL and meet ADR-0046 rule 8.
- [ ] A cast table covers every Chapter 1 speaker the answers say is voiced.
- [ ] The listening page's source is in `assets-src/audio/`.
- [ ] Dependent tickets are updated; `cargo xtask ticket-lint` passes.

## Tests required

None (docs only).

## Completion notes


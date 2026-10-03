---
id: "0042"
title: "Decide: Japanese — the font, picking a language, the machine-translation label, naming the lead"
type: design-decision
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0042 — Decide: Japanese — the font, picking a language, the machine-translation label, naming the lead

## Context

Nick, 2026-10-03 (`docs/design/voices-languages-and-script.md`): add a
machine-translated Japanese option, with a human translation later if the
game earns enough. Not a blocker for Chapter 1 or Act 1. The technical
plan is [ADR-0045](../../docs/adr/0045-languages-text-by-key-and-line-ids.md);
this ticket asks Nick the parts a player sees.

Nobody on the project reads Japanese, Nick included. So he judges the
*look* from rendered screens, and the wording questions are put to him in
English with what Japanese players are used to.

## Nick input

**Decision.** Run with the `ask-nick` skill, one question at a time, each
with options from real games and "describe your own". Expect several
rounds on the font (he picks looks by eye).

## Scope

**In:** the questions below; the answers written into
`docs/design/voices-languages-and-script.md` (a *Japanese* section) and a
row in `docs/design/README.md`; the chosen font's row in
`assets/fonts/README.md`'s candidates table (licence checked at source).

**Out (do not do):** any game code; adding the font to the atlas (0236);
translating anything (0719). No mockup made with bought art is committed
(ADR-0040); mockups use the public placeholder portraits.

## Implementation steps

1. **Find the font candidates.** 16×16 pixel fonts with kana and kanji
   whose licence ADR-0013 allows (OFL-1.1, public domain, MIT…), read at
   the source on the day. Start from: GNU Unifont (dual GPL / OFL-1.1),
   Shinonome 16 (public domain), DotGothic16 (OFL-1.1), Ark Pixel 16px
   (OFL-1.1), M+ bitmap (its own licence: needs ADR-0013's review). Drop
   any whose licence fails; say so in the ticket.
2. **Render mockups**, real pixels at 2×, of the same dialogue box and the
   same unit-info panel in each font, beside Terminus Latin text (stats
   and numbers stay Latin). A throwaway script outside the game is fine;
   put the PNGs in `docs/screenshots/0042-*.png`. Use a short Japanese
   sample of a Chapter 1 line (machine-translated for the mockup only).
3. **Ask Nick**, in this order:
   1. **The font**, from the mockups.
   2. **Picking a language.** Options: asked once on first launch, before
      the layout picker (most console ports); follows the system language
      with a row in Options (Steam's usual behaviour); Options only,
      English until changed.
   3. **Saying it is machine-translated.** Options: a note beside the
      language's name where it is picked ("日本語 (機械翻訳)"); a one-time
      notice when Japanese is first chosen; store page and credits only.
      Tell him plainly: Japanese players spot machine translation at once
      and review it harshly when it isn't labelled.
   4. **Naming the lead in Japanese.** Options: a kana grid picked with
      the cursor (Dragon Quest, Pokémon); Latin letters only (the screen
      as it is); a fixed default name with no entry in Japanese.
   5. **Names.** Katakana for every name (what Fire Emblem's Japanese
      releases do for Western-style names), or Latin names left as they
      are.
   6. **The game's title in Japanese:** keep *Visions of Shuyi* in Latin
      letters, or add a Japanese subtitle. (Feeds 0906's trademark check
      if it changes.)
   7. **Voices with Japanese text** is asked in 0043, not here.
4. Record each answer verbatim and as a rule. If an answer needs work no
   ticket covers (a kana name-entry grid), write that ticket (`write-ticket`)
   and add it to 0719's `blocked_by` if Chapter 1 in Japanese needs it.
5. Update 0236, 0718 and 0825 where an answer changes their text.

## Acceptance criteria

- [ ] Every question above has Nick's answer, verbatim, in the design doc.
- [ ] The chosen font's licence is quoted with its source URL and passes ADR-0013.
- [ ] The mockups are in `docs/screenshots/` and contain no bought art.
- [ ] Follow-up tickets exist for anything the answers add; `cargo xtask ticket-lint` passes.

## Tests required

None (docs only).

## Completion notes


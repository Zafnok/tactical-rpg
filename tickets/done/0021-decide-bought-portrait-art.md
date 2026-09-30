---
id: "0021"
title: "Decide the bought character art: portraits and battle sprites"
type: design-decision
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-30
---

# 0021 — Decide the bought character art: portraits and battle sprites

## Context

While reviewing the dialogue screen (0704, PR #77, 2026-09-28) Nick said he
doesn't like Claude's portrait art and wants bought portraits instead. After
a search of itch.io he picked the style of **CaptainSkolot's "Ultimate
Portrait Pack"** bundle (<https://itch.io/s/121612/95-off-ultimate-portrait-pack>,
45 packs, $13.99 for all in a sale ending around 2026-10-03).
His words: "I like the style better than your style", and "it's fantasy not
modern day which fits the game".

**Nick's answers so far (2026-09-28):**

- **Paid assets:** "buying is OK as long as it's not super expensive."
- **When to buy:** "I'll buy it later when we finish more stuff." He was
  told the $13.99 price is a sale ending around 2026-10-03. The page lists
  the regular total as $381.55, with single packs about $5–$15 each (the
  knight pack is $8.99). Buying only the 6–8 packs Chapter 1 would use at
  full price would cost roughly $50–$90. Before buying, check the current
  prices and recheck the licence text.
- **Coverage:** he was told plainly that the bundle covers most of Chapter
  1, but not all of it (Tamsin and Crane are gaps; see the mapping in 0706),
  and that later chapters' characters are unknown. He accepted that; no
  promise was made that no other art would be needed.

Facts about the bundle, checked on its pages on 2026-09-28:

- Art is **64×64 pixels**, not the 32×32 in ADR-0018 and
  `docs/design/look-and-feel.md`. Medieval packs (knights, soldiers, elves,
  wizard, peasants, noble lady, merchant, "magic heroes") have up to 12
  expressions, one character per pack; other packs have one face per
  character.
- Licence (the same text on every pack): free and commercial projects
  allowed, modifying allowed, credit optional; **no redistributing or
  reselling the assets on their own**.
- Each page says the art was "created using a local generative tool for
  rough concepts, then refined by hand".

This changes three recorded decisions:

- `docs/design/look-and-feel.md` § Portraits (32×32, drawn per character
  with Nick) → the bundle's 64×64 style.
- ADR-0013 allows only free art (`CC0`, `CC-BY-4.0` or our own) → paid art
  must be allowed.
- ADR-0013 "Original content only: … portraits … are created for this game"
  → bought portraits are allowed.

It must stay consistent with ticket 0020 (`docs/design/audio.md`, decided
the same day): **music and sound effects stay free** (CC0 or CC-BY 4.0),
so allowing paid assets here covers **art only** unless Nick says
otherwise. Nick also wants **every third-party work credited**, even where
the licence doesn't require it. So the bundle goes on the credits screen
(0808), although its licence makes credit optional.

**Changed 2026-09-30: battle sprites too.** Nick expects full-body art of
the two fighters on the combat screen (0011, `look-and-feel.md`; ticket
0413). Since he doesn't want Claude-drawn character art, that art has to be
bought as well. His words: "I guess we need an itch artist who has a pack
with portraits and battle sprites". **CaptainSkolot sells no full-body,
battle or map sprites** (checked on <https://captainskolot.itch.io/>,
2026-09-30: portraits, backgrounds, icons and item art only). So the
portrait choice above is open again: either one artist whose packs have both
portraits and matching battle sprites, or CaptainSkolot's portraits next to
someone else's sprites. Portraits and sprites appear on the same combat and
dialogue flow, so a style clash would show.

Technical follow-ups (not decided here): 0110 keeps bought files out of the
public repo, 0711 draws PNG portraits (64×64 if the art stays CaptainSkolot's),
0706 assigns the portraits to the cast, 0413 draws the battle sprites.

## Nick input

**Decision** (via the `ask-nick` skill). Ask:

0. **Which artist.** First search itch.io (and note the store and licence of
   each hit) for artists or packs that sell **portraits and full-body battle
   sprites of the same characters or classes, in one style**. Fire Emblem
   GBA-style packs (portrait, map sprite and battle animation per class) are
   the closest match to what Nick described; say whether a pack has map
   sprites too (ticket 1006). For each candidate, check: medieval fantasy;
   licence allows a sold game (ADR-0013 rules for art, plus this ticket's
   answers); price; which Chapter 1 classes it covers (lord, Rider, Archer,
   Cleric, Guard, Mage, Brigand, Raider, per 0413); how many animation frames
   the battle sprites have; portrait size and expressions (the dialogue
   screen needs `neutral`, `happy`, `angry`, `sad`, `surprised`). Render real
   mockups (dialogue screen and a combat scene, with the store previews
   scaled into our frames) for:
   - **A.** One artist with both portraits and battle sprites (one mockup
     per strong candidate).
   - **B.** CaptainSkolot's portraits (his pick of 2026-09-28) plus the best
     separate battle-sprite pack. Show the two side by side honestly, style
     clash and all.
   - **C.** CaptainSkolot's portraits plus battle sprites commissioned from
     CaptainSkolot (he takes commissions; ask for nothing, just name it as an
     option with an unknown price).
   - **D.** Describe your own.

   Remind Nick that the CaptainSkolot sale ($13.99 for the bundle) ends
   around 2026-10-03, and don't press him to buy.
1. Is **AI-assisted art** acceptable? (He chose the bundle knowing this, but
   didn't answer it directly.) The seller says it was. Some players
   and reviewers object. Steam asks developers to disclose AI-generated
   content on the store page. Options: accept and disclose it; buy only
   hand-made art (which rules this bundle out); describe your own.
2. **Paid assets in general:** answered above ("OK as long as it's not super
   expensive"). Ask only for a rough price ceiling per purchase, so "super
   expensive" can be checked without asking him each time.
3. **Characters the chosen packs don't cover** (with CaptainSkolot: Tamsin
   and Crane in Chapter 1, see 0706; redo the check for another artist):
   options are commissioning the same artist (keeps the style consistent), changing the character's
   portrait brief to fit a bought face (a story change), or having Claude
   make small pixel edits to a bought face (recolours, a scar, a changed
   expression; not new hair or clothes).

Nick buys the packs himself (Claude doesn't make purchases) and puts the
downloaded zips where 0110 says.

## Scope

**In:** the artist search and mockups; record the answers; update the
design doc and supersede the parts of ADR-0013 above. If the artist changes
from CaptainSkolot, rewrite 0706's mapping table for the new packs and fix
the 64×64 numbers in 0711 (with the `write-ticket` skill's rules), and
update the portrait memory note.

**Out (do not do):** code, importing any art, the private-assets setup
(0110), the renderer change (0711), the combat scene (0413), buying
anything.

## Implementation steps

1. Search itch.io for question 0 and build the mockups (`ascii-art` skill
   for the frames; store preview images only as mockup input, never
   committed).
2. Run `ask-nick` with questions 0–3.
3. `docs/design/look-and-feel.md` § Portraits: replace the "32×32, drawn per
   character with Nick" rules with the bought style (size, artist, packs,
   the answers above). Add the battle-sprite choice next to the "Combat
   screen" bullet (the scene's own details stay with 0413). Quote Nick's words in § Nick's words. Keep the speaker
   and listener rules. Keep the rule that there's no portrait in the battle
   panel.
4. New ADR (`write-adr` skill), superseding ADR-0013's asset rules: paid
   **art** is allowed when its licence permits use in a sold game (audio
   stays free, per `audio.md`). Bought art is credited like everything
   else (`audio.md` rule 3, credits screen 0808).
   Files whose licence forbids redistribution live in the private assets
   repo (0110), never in this public repo. Each bought asset is listed in
   `THIRD_PARTY_ASSETS.md` (name, seller, URL, licence text, date bought,
   whether it's private). Add a "superseded in part by" note to ADR-0013's
   status line (the only edit allowed on an accepted ADR) and to
   `docs/adr/README.md`.
5. Update ADR-0018's status line the same way for its portrait section, and
   point it to 0711's ADR.
6. Update the `ascii-art` skill's portrait section: portraits and battle
   sprites are bought, and Claude doesn't draw them any more (except small edits if Nick allows them
   in question 3).

## Acceptance criteria

- [x] Nick saw mockups for each option in question 0 and picked one.
- [x] `look-and-feel.md` § Portraits describes the bought style (artist,
      size) and Nick's answers, including the battle-sprite source.
- [x] If the artist isn't CaptainSkolot: 0706 and 0711 updated to match.
- [x] New ADR accepted; ADR-0013 and ADR-0018 status lines point to it.
- [x] `ascii-art` skill updated.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None (docs only). `cargo xtask ticket-lint` and `typos` pass.

## Completion notes

- **Search:** itch.io, 2026-09-30. There's no original, commercially licensed
  Fire Emblem GBA-style class pack; FE-style art is fan-ripped (disqualified)
  or AI-made. Nothing mounted (Rider) exists in any candidate. Mockups
  (store previews in the 0704 dialogue screen plus a stand-in combat
  scene) were rendered in the scratchpad and not committed: A1 SolaarNoble,
  A2 Tiny Tales (Mega Tiles), A3 Time Fantasy, B1/B2 CaptainSkolot faces
  plus Time Fantasy or SolaarNoble battlers. C (commission CaptainSkolot)
  was named without a mockup. Not mocked up: Holder (free, credit
  required), HEROES 99 (32×32 faces).
- **Nick's answers:**
  - Artist: **A2, Mega Tiles' Tiny Tales**.
  - After being shown that the big Tiny Tales fighters are still images and
    only the small ones animate: **big still images moved by the game**
    (1A).
  - AI-assisted art is OK only with a human touch.
  - No price ceiling: he decides each purchase.
  - Gaps: Character Generator EX or Claude's small edits.
  - Classes with no hero art: still images as stand-ins.

  Recorded in `look-and-feel.md` § *Portraits and battle art (bought)*.
- **ADR-0032** (bought art allowed, audio stays free, private files,
  `THIRD_PARTY_ASSETS.md` rows, credit everything). ADR-0013 and ADR-0018
  status lines point to it.
- **Tickets updated:**
  - 0706: new mapping table for Tiny Tales.
  - 0711: renamed to `0711-png-portraits.md`; 48×48 faces at 5 px per
    pixel; importer slices the 4×2 face set.
  - 0413: still images moved by code; the scene's remaining questions.
  - 0110, 0029, 1006: point to the new packs.
- **Claude's decisions (technical):** dialogue uses the face set faces (8
  expressions), not the one-expression large portraits. Faces are drawn at
  the largest whole scale that fits the existing frame.
- **Claude's starting rule** (gameplay-facing, for Nick to veto): the
  Brigand stand-in is the orc axe fighter, but Chapter 1's bandits **stay
  human in the story**. Nick accepted "3A" without choosing between that
  and making the bandits orcs.
- **Before buying:** recheck prices and licence text (nothing was on sale on
  2026-09-30; the Mega Tiles bundle is $99.99 for 37 packs), and check the
  Character Generator's licence for generated characters.
- No follow-up tickets created: the open questions belong to 0413, 0706
  and 1006, which were updated.

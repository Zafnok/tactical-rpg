---
id: "0021"
title: Record the switch to bought portrait art (CaptainSkolot bundle)
type: design-decision
milestone: M6 Story & dialogue
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0021 — Record the switch to bought portrait art

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

Technical follow-ups (not decided here): 0110 keeps bought files out of the
public repo, 0711 draws 64×64 PNG portraits, 0706 assigns the portraits to
the cast.

## Nick input

**Decision** (via the `ask-nick` skill). His style choice above is already
made. Ask only what's still open:

1. Is **AI-assisted art** acceptable? (He chose the bundle knowing this, but
   didn't answer it directly.) The seller says it was. Some players
   and reviewers object. Steam asks developers to disclose AI-generated
   content on the store page. Options: accept and disclose it; buy only
   hand-made art (which rules this bundle out); describe your own.
2. **Paid assets in general:** answered above ("OK as long as it's not super
   expensive"). Ask only for a rough price ceiling per purchase, so "super
   expensive" can be checked without asking him each time.
3. **Characters the bundle doesn't cover** (Tamsin and Crane in Chapter 1;
   see 0706): options are commissioning the same artist (CaptainSkolot takes
   commissions, which keeps the style consistent), changing the character's
   portrait brief to fit a bought face (a story change), or having Claude
   make small pixel edits to a bought face (recolours, a scar, a changed
   expression; not new hair or clothes).

Nick buys the bundle himself (Claude doesn't make purchases) and puts the
downloaded zips where 0110 says.

## Scope

**In:** record the answers; update the design doc and supersede the parts of
ADR-0013 above.

**Out (do not do):** code, importing any art, the private-assets setup
(0110), the renderer change (0711).

## Implementation steps

1. Run `ask-nick` with the three questions above.
2. `docs/design/look-and-feel.md` § Portraits: replace the "32×32, drawn per
   character with Nick" rules with the bought style (64×64, the bundle, the
   answers above). Quote Nick's words in § Nick's words. Keep the speaker
   and listener rules. Keep the rule that there's no portrait in the battle
   panel.
3. New ADR (`write-adr` skill), superseding ADR-0013's asset rules: paid
   **art** is allowed when its licence permits use in a sold game (audio
   stays free, per `audio.md`). Bought art is credited like everything
   else (`audio.md` rule 3, credits screen 0808).
   Files whose licence forbids redistribution live in the private assets
   repo (0110), never in this public repo. Each bought asset is listed in
   `THIRD_PARTY_ASSETS.md` (name, seller, URL, licence text, date bought,
   whether it's private). Add a "superseded in part by" note to ADR-0013's
   status line (the only edit allowed on an accepted ADR) and to
   `docs/adr/README.md`.
4. Update ADR-0018's status line the same way for its portrait section, and
   point it to 0711's ADR.
5. Update the `ascii-art` skill's portrait section: portraits are bought, and
   Claude doesn't draw them any more (except small edits if Nick allows them
   in question 3).

## Acceptance criteria

- [ ] `look-and-feel.md` § Portraits describes the bought 64×64 style and
      Nick's answers.
- [ ] New ADR accepted; ADR-0013 and ADR-0018 status lines point to it.
- [ ] `ascii-art` skill updated.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (docs only). `cargo xtask ticket-lint` and `typos` pass.

## Completion notes


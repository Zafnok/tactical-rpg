# ADR-0032: Bought art is allowed; audio stays free

- **Status:** Accepted
- **Date:** 2026-09-30
- **Related tickets:** 0021, 0110, 0413, 0706, 0711, 0808, 0903
- **Supersedes in part:** ADR-0013 (§2 "Art / audio" licences and "payment"
  rule for art; §3 "Original content only" for portraits and battle art)

## Context

ADR-0013 allows only free third-party art and audio (`CC0-1.0`,
`CC-BY-4.0`, `CC-BY-3.0` via ADR-0027, or our own), denies anything that costs
money, and says portraits are "created for this game". In ticket 0021 Nick
dropped Claude-drawn portraits and chose to **buy** character art: dialogue
faces and combat-screen battle images from Mega Tiles' Tiny Tales packs on
itch.io (`docs/design/look-and-feel.md`, *Portraits and battle art*). Music
and sound effects stay free (`docs/design/audio.md`, ticket 0020).

Store asset licences are custom texts, not SPDX licences. The Tiny Tales one
(identical on every Mega Tiles page, read 2026-09-30): "You cannot claim
ownership of the assets (copyright/IP). Assets can be used both in free and
commercial games. Assets can be modified freely to fit the needs of your
game. Redistribution and reselling of the asset files or derivatives as is
without permission is strictly forbidden." Our repository is public, so
committing those files would redistribute them.

## Decision

1. **Paid art is allowed** (images only: portraits, battle images, map
   sprites, backgrounds) when its licence, read on the store page on the day
   of purchase, allows:
   - use in a **commercial** game,
   - **modification** (we recolour and crop),
   - with **no royalties, revenue share or per-copy fees** (a one-off price
     is fine).

   It must not be copyleft or non-commercial (ADR-0013's denied list still
   applies), and must not be ripped from another game. **Audio stays free**:
   ADR-0013/0027's audio licences are unchanged.
2. **AI-assisted art** is allowed only when a human worked on it (Nick's
   rule, `look-and-feel.md`). Pure generator output is not. If any shipped
   art is AI-assisted, the Steam page discloses it (0903), and the
   `THIRD_PARTY_ASSETS.md` row says so.
3. **Bought portraits and battle art are allowed** in place of "created for
   this game". Maps, story text and names stay original.
4. **Files whose licence forbids redistribution never enter this public
   repo.** They live in the private assets repository (0110), and the game
   builds and passes every gate without them, using public placeholders.
5. **Every bought asset gets a row in `THIRD_PARTY_ASSETS.md`**: item,
   seller, store URL, the licence text as quoted on the page, date bought,
   whether it's AI-assisted, and *private* (not in this repo). The licence
   text is saved next to the files in the private repo.
6. **Credit every bought work** on the credits screen (0808), even when the
   licence makes credit optional (`audio.md` rule 3).
7. **Nick buys; Claude never purchases.** Before each purchase Claude
   rechecks the price and licence text and tells Nick; Nick decides whether
   the price is fine (he asked not to be given a ceiling).

## Consequences

- The dialogue and combat screens can use art Nick likes, at a one-off cost.
- A clone of the public repo shows placeholders instead of the bought art.
  The Pages build and release builds need the private repo (0110).
- Custom licences can't be checked by `cargo-deny`; the check is manual, at
  purchase time, and recorded in `THIRD_PARTY_ASSETS.md`.
- Modded or forked copies can't ship the bought art. That's expected.

## Alternatives considered

- **Keep ADR-0013 (free or our own art only)**: Nick doesn't want
  Claude-drawn character art, and free packs with matching faces and battle
  images in one style weren't found.
- **Commit bought files to the public repo**: breaks the "no
  redistribution" clause of the licences.
- **Allow paid audio too**: Nick decided audio stays free (0020).

---
id: "0906"
title: "Check the title for trademark conflicts before the Steam page goes up"
type: research
milestone: M8 Release
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 0906 — Check the title for trademark conflicts before the Steam page goes up

## Context

The title is **Visions of Shuyi** (`docs/design/title.md`, ticket 0012). On
2026-09-30 Nick asked whether it could get us in trouble with Square Enix's
*Visions of Mana* (2024). The findings (see "Trademark risk" in
`docs/design/title.md`):

- Titles can't be copyrighted; the question is trademark (likelihood of
  confusion).
- The overlap: same "Visions of ___" pattern, same genre (fantasy RPG), and in
  our lore Shuyi is a guardian providing a lifeblood "something like mana or
  qi", which echoes the Mana Goddess / Mana Tree.
- A quick search found no *released* game by another studio named
  "Visions of [something]". *Visions of Evil* (Steam app 1550130) is
  unreleased. So there is no clear precedent either way.
- Nick weighed switching to another word from the shortlist (Vows / Veil /
  Voice of Shuyi, all still "VoS") and chose to **keep Visions of Shuyi**. The
  accepted worst case is a forced rename after a cease-and-desist letter.

This ticket is the check before money and marketing are committed to the
name. Do it before the Steam store page is created (0903) or any public
marketing push. It may be done earlier; it must not be skipped.

## Nick input

**Sign-off:** read the one-page summary in the PR and decide: keep the title,
pay for a lawyer's clearance search, or reopen the title decision (a new
`00xx` ticket, run with `ask-nick`). If Nick wants a lawyer, that's his
money and his call; the session only explains what to ask for.

## Scope

**In:**
- Searching public trademark registers and stores for conflicts with
  "Visions of Shuyi", "Shuyi" and "VoS".
- Checking our own player/shopper-facing text against the precautions in
  `docs/design/title.md`.
- Writing up findings in `docs/release/title-trademark-check.md`.

**Out (do not do):**
- Do not rename anything or change the title. Only Nick can reopen that.
- Do not file a trademark application or contact Square Enix or a lawyer.
- Do not give legal conclusions as certain. State what was searched and found.

## Implementation steps

1. Search the USPTO trademark search (<https://tmsearch.uspto.gov>) for live
   marks containing `VISIONS OF`, `SHUYI` and `VOS` in class 9 (downloadable
   games) and class 41 (online game services). Note owner, mark, status and
   class for each hit. Specifically record Square Enix's "VISIONS OF MANA"
   and any "VISIONS" mark on its own.
2. Search the EUIPO (<https://euipo.europa.eu/eSearch>) and WIPO Global Brand
   Database (<https://branddb.wipo.int>) the same way.
3. Search Steam, itch.io, GOG and the console stores for games titled
   "Visions of …" or containing "Shuyi". Record for each whether it is
   released and who publishes it. Verify release status on the store page,
   not from search snippets.
4. Grep the repo's player-facing text (`assets/`, store/marketing drafts under
   `docs/`, `README.md`) for "mana" used to describe Shuyi, and check the logo
   / title-screen art for anything resembling the Mana series logo (sword or
   tree motif built into the lettering). List any hits.
5. Write `docs/release/title-trademark-check.md`: date, what was searched,
   hits table, precaution check results, and a plain-language summary for
   Nick with the three options from "Nick input".

## Acceptance criteria

- [ ] `docs/release/title-trademark-check.md` exists with the searches from
      steps 1–3 (each with the date searched and a link) and a hits table.
- [ ] Every "Visions of …" game listed says whether it's released, checked on
      its store page.
- [ ] Step 4's precaution check is recorded (hits or "none found").
- [ ] The PR description has a plain-language summary for Nick with the three
      options.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None (research/doc only). Gates still run.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*

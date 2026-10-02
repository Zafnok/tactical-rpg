---
id: "0037"
title: "Decide: more building tiles (village, gate, throne), capturing and healing tiles"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0804"]
nick_input: decision
completed:
---

# 0037 — Decide: more building tiles, capturing and healing tiles

## Context

`docs/design/terrain.md` (decided in 0301, 2026-09-26) has one building
tile, `fort`, and lists this as an open sub-question: "Capturing, healing
tiles and more building tiles (village, gate, throne, …) (Nick, Q5 and Q7),
after the first playtest." No ticket carried it, and ticket 0313 (villages
and the Visit action) sat at `status: blocked` waiting for it. Found while
checking ticket dependencies (2026-10-01).

Nick's words then:

> **Q5. Healing tiles:** "2 I think we can defer healing for now, since the
> idea of capturing sounds interesting, but I also don't think I ever
> explicitly signed off on a fort, gate, or throne tile type either."
>
> **Q7. Building tiles:** "we can go w C for now. make a note that we might
> allow capturing later and the other building type tiles might come around
> then or even without capturing. but let's keep it simple for the first
> playtest."

What waits on the answer:

- **0313** (core rules for villages) and the village part of **0409**.
  The village rule itself is already written as a starting rule
  (`docs/design/weapons-and-items.md`, *Money and shops*): a player unit on
  the village gate uses `Visit` for a one-time gift, then the village
  closes. Only the tile is missing.
- Objectives: 0801's battle file has `Seize((x, y))`, and 0803 puts the
  Chapter 1 boss on a fort because there is no gate or throne.
- The terrain data keeps a `heal_percent` field, 0 on every terrain.

**Changed 2026-10-02 (ticket 0038):** the battle map is now drawn with
the bought Tiny Tales tilesets (0437), and the glyph look stays as the
public placeholder. So each new tile needs **both** looks. The bought
World Map set has single-tile villages, towns, castles, towers and cave
mouths (`Set_C_Icons`) and walled towns two tiles square; the Dungeons
sets have doors, gates, stairs and chests; nothing in the bundle is a
throne. Show the mockups in both looks, and say which new tile has no
bought picture.

## Nick input

**Decision**, with the `ask-nick` skill, after the Chapter 1 playtest
(0804), as he asked. Show map mockups (`ascii-art` skill) of each new tile
next to `fort` (`╦╦`); a terrain glyph must never look like the cursor
(`look-and-feel.md`). At most three questions per message:

1. **Which building tiles are added now?** Village, gate, throne, each or
   none, with what each does in Fire Emblem GBA (village: visit for a gift;
   gate and throne: the boss stands there, seize to win, strong defence)
   and its movement cost and Def/Avoid, in `terrain.md`'s tables.
2. **Capturing:** the Advance Wars idea he found interesting (a unit stands
   on a building for a turn or two and it becomes yours), FE's seize only,
   or not yet. Say plainly what capturing would add to a battle (income,
   healing, a win condition) and what it costs in rules and AI.
3. **Healing tiles:** which tiles heal, how much, and whose units (FE GBA:
   forts, gates and thrones heal whoever stands there / Advance Wars: only
   buildings you own / none).
4. **Enemies destroying villages** (FE): yes, no, later.

## Scope

**In:**
- The decision, recorded in `docs/design/terrain.md` (rules, numbers marked
  Nick's or *tunable*, his words) and the `docs/design/README.md` table.
- Updating 0313 and 0409 to match. If there is no village tile, close 0313
  and remove the village part of 0409.
- Implementation tickets (`write-ticket`) for whatever else he chose (new
  tiles, capturing, healing), each blocked by this one.

**Out (do not do):**
- Any code or data.
- Changing Chapter 1's map.

## Implementation steps

1. Read `terrain.md`, `weapons-and-items.md` (*Money and shops*),
   `chapter-1.md` and Nick's playtest notes in 0804 for anything about
   terrain or buildings.
2. Render the mockups; ask the questions; iterate.
3. Record the answers; update 0313, 0409 and the design README; write the
   implementation tickets.

## Acceptance criteria

- [ ] `terrain.md` answers the open sub-question with Nick's words and
      concrete rules, or records that he deferred it again (and until
      when).
- [ ] 0313 and 0409 match the answer.
- [ ] Implementation tickets exist for everything he chose.
- [ ] `cargo xtask ticket-lint` passes.

## Tests required

- None (documentation and tickets only).

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*

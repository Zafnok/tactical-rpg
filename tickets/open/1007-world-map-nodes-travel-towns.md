---
id: "1007"
title: "World map: nodes, travel, towns, save and camp"
type: feature
milestone: Post–Chapter 1
model: opus-5.5
effort: high
status: todo
blocked_by: ["0801", "0802", "0409", "1003"]
nick_input: sign-off
completed:
---

# 1007 — World map: nodes, travel, towns, save and camp

## Context

[`docs/design/world-structure.md`](../../docs/design/world-structure.md)
(ticket 0008): Chapters 1–2 (maybe 3) are linear. After that, each act has a
**world map** in the style of FE Sacred Stones. The army moves between nodes.
Story battles open new paths. Towns have shops. The world map menu offers
`Save` anywhere and `Camp`. A chapter is a story beat that plays out over
several nodes and ends with its story battle (`chapter-1.md`).

This ticket builds the map itself: nodes, paths, travel, story-battle nodes,
side-quest nodes, towns, and the map menu. **Skirmishes** (fixed and random)
are 1008. It builds on the chapter format and campaign state from 0801, saving
from 0802, the shop screen from 0409 and the camp screen from 1003.

## Nick input

**Sign-off:** before any code, show Nick an ASCII mockup of a world map
(`ascii-art` skill) with every node kind and its symbol and level marker. Nick
asked for random and fixed battles to be "clearly delineated… Some symbol
above them or a level marker or both", and the design says both. Offer a few
glyph/colour options and expect several rounds. When the ticket is done, Nick
plays a test map: travels, visits a town, saves and loads on the map, opens
camp, and starts a story battle.

## Scope

**In:**
- A world-map data format (`assets/worldmaps/*.ron`): nodes (id, position,
  kind, glyph override), paths between nodes, the act it belongs to, the
  enemy-level range for the act, and unlock conditions (story flags, e.g.
  "story battle ch04 won").
- Node kinds from the design: `StoryBattle`, `SideQuest`, `Town`, `Empty`.
  Leave room for `FixedSkirmish` / `RandomSkirmish`, but don't implement them
  (1008).
- `core` state for the world map in the `Campaign`: the current act's map id,
  the army's node, cleared nodes, unlocked paths. Changes go only through
  `Command`s → `Event`s (ADR-0004).
- Chapters that live on the world map: extend the 0801 chapter file so a
  chapter can say "world-map chapter: these nodes open, this story battle
  ends it". Linear chapters keep working unchanged.
- The world-map screen: draw nodes, paths and the army marker. Move the
  virtual cursor node to node, and travel along open paths. Show a symbol and
  a level marker above each battle node. Hover info shows the node name, kind
  and level.
- Entering a battle node runs its scenes and battle through the normal flow
  (0801), then comes back to the world map.
- Towns: entering one opens the 0409 shop screen with that town's inventory.
- World map menu: `Save` (0802 slots, any time no battle is running), `Camp`
  (1003 screen), `Unit` / `Items` if they exist, and `Options`.
- Moving to a new act: when an act's final story battle is won, the next
  act's world map replaces the current one. The old map can't be reached
  again.
- A small test world map in the data (a few nodes of each kind in this ticket)
  so the screens and tests have something to use. It must not be real story
  content.

**Out (do not do):**
- Fixed and random skirmishes, their markers and the level cap (1008).
- Real world maps, story battles or side quests for Act 1 (story/content
  tickets after 0701).
- Recruitable wanderers in towns, the grinding tower, branching routes (all
  deferred in `world-structure.md`).
- Materials and forging.

## Implementation steps

1. Read `world-structure.md`, `chapter-1.md`, `death-and-difficulty.md`
   (*Saving*), `supports.md` (*Viewing support conversations*), and the code
   from 0801, 0802, 0409 and 1003.
2. Get Nick's sign-off on the node-glyph mockup first (see Nick input). Record
   the chosen glyphs/colours in `docs/design/look-and-feel.md`.
3. `core::worldmap`: data types, loader + content validation (every path
   joins existing nodes, every node is reachable once all its unlock
   conditions hold, each act has exactly one final story battle). Add
   commands `TravelTo(node)`, `EnterNode`, and events for them.
4. Extend `Campaign` with world-map state, and make sure the 0802 `SaveFile`
   saves and loads it. Bump `SAVE_VERSION` if the format changes.
5. Extend the chapter format for world-map chapters (document it in
   `assets/chapters/README.md`).
6. `ui` world-map screen with cursor, travel, hover info and markers, then
   the map menu (Save/Camp/…) and town → shop.
7. Wire it into `ui::flow`: linear chapters → world map → battle → back to
   the map → next act.
8. Add the test world map and tests.

## Acceptance criteria

- [ ] Nick signed off on the node glyph mockup; the choice is in `look-and-feel.md`.
- [ ] Travel only follows open paths; locked nodes can't be entered (tests).
- [ ] Winning a story battle opens its paths and marks the node cleared.
- [ ] `Save` on the world map → `Load Game` → same act, node and cleared nodes (harness test).
- [ ] `Camp` and towns open from the world map.
- [ ] Winning an act's final story battle loads the next act's map; the old one is gone.
- [ ] Linear chapters (Chapter 1) still play exactly as before (existing tests pass).
- [ ] Nick played the test map and signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: path/unlock rules, travel commands, act change, content validation errors.
- Property: after any sequence of valid `TravelTo` commands, the army is always on a reachable, unlocked node.
- Snapshot: world-map screen (markers, hover info), world map menu.
- Integration: scripted keys travel → town → shop → leave → Save → reload → same state; travel → story battle → win → back on the map with the new path open.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*

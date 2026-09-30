# Chapter files (`.ron`) and the New Game file

A chapter is a story beat (`docs/design/chapter-1.md`): scenes, a battle,
more scenes, then the next chapter. Each `*.ron` file here is one chapter,
loaded by `trpg_content::chapter` and validated by the all-assets test
(ADR-0032). The file stem is the chapter's id. The game flow
(`trpg_ui::flow`) plays them.

```ron
(
    id: "ch01",
    title: "Chapter 1: …",
    intro_scenes: ["ch01_intro", "ch01_prebattle"],
    battle: "ch01",                     // assets/battles/ch01.ron
    victory_scenes: ["ch01_victory", "ch01_tbc"],
    next: Some("ch02"),                 // or None: "To be continued"
)
```

| Field | Meaning |
| ----- | ------- |
| `id` | Must match the file name. |
| `title` | The chapter's title (for the title card, 0812, and save slots, 0802). |
| `intro_scenes` | Scene ids (`assets/dialogue/`) played in order before the battle. Default: none. |
| `battle` | The battle's id (`assets/battles/`). |
| `victory_scenes` | Scenes played in order after a victory. Default: none. |
| `next` | The next chapter's id, or `None`: the game shows "To be continued" and returns to the title. |

A chapter is one battle for now; chapters with several battles come with
the world map (1007, 1008).

The flow: intro scenes → the battle → on a victory, the result is applied
to the army (`Campaign::apply_result`) → victory scenes → (the save prompt,
0802) → `next`. On a defeat: Game Over, with `Retry` (the battle again from
its first turn, every rewind charge back) or `Title`.

Checked: the id matches the file name; every scene exists; the battle
exists; `next` names a chapter.

## New Game (`assets/data/new_game.ron`)

```ron
(
    first_chapter: "ch01",
    roster: ["lead", "bors", "mira"],   // characters, in roster order
    gold: 0,
    stock: [],                          // item ids, one entry per item
)
```

Checked: the first chapter exists; every roster character exists, once,
and the lead (`"lead"`) is among them; every stock item exists. The lead's
unit takes the name the player gives it at New Game.

`test.ron` (the test chapter New Game plays until Chapter 1, 0803) and
`quick.ron` (the debug Quick Battle, no scenes) are placeholders.

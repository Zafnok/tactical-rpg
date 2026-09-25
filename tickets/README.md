# Tickets

The backlog lives here as Markdown files. Full rationale:
[ADR-0010](../docs/adr/0010-ticket-workflow-and-model-routing.md).

```
tickets/
  README.md     ← you are here
  TEMPLATE.md   ← copy for new tickets (see the write-ticket skill)
  open/         ← todo / in-progress / blocked
  done/         ← archived; moved here by the PR that completes the ticket
```

## Numbering

`NNNN-short-kebab-title.md`. The hundreds digit is the block:

| Block | Milestone | Notes |
| ----- | --------- | ----- |
| `00xx` | **Design decisions (Nick)** | Questions with options; answers go to `docs/design/` |
| `01xx` | M0 Foundation | Workspace, CI, scanners, SonarCloud, mutation, release plumbing |
| `02xx` | M1 Engine | Glyph buffer, font, input, screens, web build, storage |
| `03xx` | M2 Core rules | Content loading, map, units, movement, combat, turns, items |
| `04xx` | M3 Battle UI | Map view, cursor, move/attack flow, forecast, info screens |
| `05xx` | M4 Enemy AI | |
| `06xx` | M5 Progression | EXP, level up, class change |
| `07xx` | M6 Story & dialogue | Story bible, dialogue engine, portraits, Chapter 1 script |
| `08xx` | M7 Chapter 1 & game flow | Flow, save/load, Chapter 1 map, playtest, options |
| `09xx` | M8 Release | itch.io, Windows polish, Steam |
| `10xx`+ | Post–Chapter 1 | Created as needed |

New tickets (including bugs) take the next free number in their block. Numbers
are never reused.

## Frontmatter

```yaml
---
id: "0303"
title: Movement range and pathfinding
type: feature        # feature | infra | design-decision | content | bug | tuning | research | playtest
milestone: M2 Core rules
model: opus-5.5      # haiku-4.5 | sonnet-5 | opus-5.5 | fable-5.1
effort: high         # low | medium | high | xhigh | max
status: todo         # todo | in-progress | blocked | done
blocked_by: ["0302"] # ticket ids that must be in done/ first
nick_input: none     # none | decision | answer-first | setup | sign-off
completed:           # YYYY-MM-DD when done
---
```

## Picking the next ticket

A ticket is **ready** when `status: todo` and every id in `blocked_by` has a
file in `done/`. Prefer tickets on the Chapter 1 critical path in
[`docs/ROADMAP.md`](../docs/ROADMAP.md). Tickets without dependencies on each
other can run in parallel sessions (use separate git worktrees).

## Working a ticket

Use the **`work-ticket`** skill. Summary: branch `t<NNNN>-<slug>` → implement
within scope → run gates → fill in Completion notes → `git mv` to `done/` →
PR titled `[NNNN] Title`.

## Nick input legend

- `none`: no Nick involvement.
- `decision`: this ticket *is* a question for Nick (the `00xx` block, run with the `ask-nick` skill).
- `answer-first`: blocked on a `00xx` design decision.
- `setup`: Nick must create an account or secret; the ticket lists exact steps.
- `sign-off`: Nick looks at or plays the result and approves or comments.

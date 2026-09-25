# ADR-0010: Ticket workflow and model routing

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

Nick drives the project by running one Claude session per ticket. Sessions may
use different models; cheaper models must be able to finish a ticket without
guessing. Finished tickets must be archived in a separate folder.

## Decision

### Where tickets live

```
tickets/
  README.md      how tickets work (read this first)
  TEMPLATE.md    copy this for new tickets
  open/          tickets not yet done (status: todo | in-progress | blocked)
  done/          archived tickets — moved here in the PR that completes them
```

Tickets are Markdown files in the repo, **not** GitHub Issues: they're
versioned with the code, readable offline by any session, and archiving is a
`git mv` inside the same PR.

### Numbering

`NNNN-short-kebab-title.md`. The **hundreds digit is the milestone block**:

| Block | Milestone |
| ----- | --------- |
| `00xx` | Design decisions — questions for Nick |
| `01xx` | M0 Foundation: workspace, CI, gates, release plumbing |
| `02xx` | M1 Engine: glyph rendering, font, input, screens, web build |
| `03xx` | M2 Core rules: map, units, movement, combat, turns |
| `04xx` | M3 Battle UI: cursor, ranges, menus, forecast, combat playback |
| `05xx` | M4 Enemy AI |
| `06xx` | M5 Progression: EXP, level ups, class change |
| `07xx` | M6 Story & dialogue |
| `08xx` | M7 Chapter 1 & game flow |
| `09xx` | M8 Release: itch, Windows polish, Steam |
| `10xx`+ | Post–Chapter 1 features (created later) |

New tickets take the next free number in the block their work belongs to
(including bugs: a combat bug found in playtest becomes e.g. `0312`). Numbers
are never reused or renumbered. Gaps are fine.

### Lifecycle

1. **Pick** a ticket in `tickets/open/` with `status: todo` whose `blocked_by`
   tickets are all in `tickets/done/`.
2. **Start:** branch `t<NNNN>-<slug>` (e.g. `t0304-pathfinding`), set
   `status: in-progress`.
3. **Work** only within the ticket's scope. Anything else discovered becomes a
   new ticket file (use the `write-ticket` skill), not extra code.
4. **Finish:** all acceptance criteria checked, local gates pass, fill in the
   ticket's "Completion notes", `git mv` it to `tickets/done/`, set
   `status: done`, open a PR titled `[NNNN] Title`.
5. **Merge** when CI is green. One ticket = one PR.

The `work-ticket` skill walks a session through this.

### Model routing

Each ticket's frontmatter names a recommended `model` and `effort`:

| Model / effort | Use for |
| -------------- | ------- |
| `haiku-4.5` / `low` | Purely mechanical: renames, moving files, bumping a value in data |
| `sonnet-5` / `medium` | Well-specified infra and CI, screens that follow an existing pattern, content data entry, small bug fixes with a clear repro |
| `opus-5.5` / `medium` | New modules with a clear spec; facilitating Nick's design-decision sessions |
| `opus-5.5` / `high` | Core rules with tricky invariants (pathfinding, combat, turn system, AI, dialogue engine), architectural work, anything touching many crates |
| `fable-5.1` / `high` | Creative writing where quality is the product: story bible, chapter scripts, character voice; ASCII portrait art direction |

The recommendation is a floor, not a ceiling: if a cheaper model gets stuck
twice, rerun with the next tier.

### Nick's involvement

Each ticket has a **Nick input** section with one of:

- `None.`
- **Answer first** — the ticket is blocked on a `00xx` design decision.
- **Setup** — Nick must do an account/secret step (exact clicks listed).
- **Sign-off** — Nick looks at or plays the result and approves or comments.

Nick is **never** asked technical questions (languages, libraries, patterns).
Game-design questions go through the `ask-nick` skill: two to four options
grounded in real games (Fire Emblem, Final Fantasy Tactics, Tactics Ogre,
Advance Wars, Triangle Strategy, XCOM, Into the Breach…), each with a
one-line "how it feels", a recommendation, and an explicit "or describe your
own" option.

## Consequences

- Any session can orient itself from `CLAUDE.md` → `tickets/README.md` → the
  ticket file alone.
- `tickets/done/` doubles as a changelog of what was built and why.

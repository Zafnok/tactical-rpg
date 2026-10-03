---
name: work-ticket
description: Pick up, implement, verify, archive and open a PR for exactly one ticket from tickets/open/. Use whenever asked to "work ticket NNNN", "do the next ticket", or start any implementation work in this repo.
---

# Work a ticket

One ticket = one branch = one PR. Do **only** what the ticket says. This repo's
owner (Nick) does not review code; CI and this checklist are the review, so
discipline here is the whole quality system.

## 0. Orient (every session, ~2 minutes)

1. Read `CLAUDE.md`.
2. Read `tickets/README.md`.
3. Read the ticket file fully, then every ADR and design doc it links.

## 1. Choose the ticket

- If the user named a ticket, use it.
- Otherwise list candidates: tickets in `tickets/open/` with `status: todo` whose
  every `blocked_by` id exists in `tickets/done/`. Prefer the lowest number on
  the Chapter 1 critical path in `docs/ROADMAP.md`. Tell the user which one you
  picked and why in one sentence.
- If the ticket's **Nick input** says *Answer first* and the design doc it needs
  doesn't exist yet in `docs/design/`, stop and tell the user which `00xx`
  ticket must be answered first. Do not guess game-design answers.

## 1b. Get the design unknowns out first (Nick asked for this)

Nick's turns are expensive; a question answered late costs a full
implement → test → gates → push cycle. Before writing code:

1. List every gameplay rule the ticket needs that `docs/design/` doesn't
   settle, and every conflict you can see between the ticket, the design
   docs and the data (e.g. a skill locked to a weapon its class can't
   wield). Check new answers against the data the same way.
2. Ask them all at once with the `ask-nick` skill, scoped to this ticket,
   before implementing. Only a rule too small to matter goes in as a
   *Claude's starting rule*.
3. When Nick's reply leaves a conflict or ambiguity, ask about it **before**
   making the rest of the changes, not after the gates have run.

## 2. Start

```bash
git switch main && git pull --ff-only
git switch -c t<NNNN>-<slug>
```

Then free disk space:

```bash
cargo xtask clean-merged-targets
```

It deletes the `target/` build folder of every other worktree whose PR has
merged (never a worktree or a branch). If this command fails, carry on: it
never blocks the ticket.

Set `status: in-progress` in the ticket frontmatter.

## 3. Implement

- Follow the ticket's **Implementation steps** in order. If a step turns out to
  be wrong, deviate minimally and explain it in the Completion notes.
- Respect crate boundaries (ADR-0004): no I/O, clock or macroquad outside `app`;
  rules only in `core`; state changes only via `Command`s.
- Touching input, controls, help bars, tips or any text that names a key:
  follow the `keyboard-input` skill (never hard-code a key).
- Drawing a picture, or anything on the battle map: follow ADR-0038
  (graphics are a skin). Pictures are sprite items from asset files; map
  things go in the map scene and are painted by each map skin; nothing
  about the look goes in `core`, the bots or play records; a test of what
  happened reads the scene or the state, not cells or colours.
- Write the tests the ticket lists (ADR-0007): unit + property for `core`,
  snapshot + scripted integration for screens.
- A test that checks what happened reads the scene or the state. Only a
  test of a look reads cells, colours or items, and it lives with the
  skin (`crates/ui/src/map_view/glyph*`). Harness shortcuts:
  `cursor_tile()`, `unit_at(pos)`, `tints_at(pos)`, `path()`
  (`crates/ui/README.md`).
- New integration tests go in the crate's `tests/it/` as a module listed in
  `main.rs`; never add a file directly under `tests/` (each one is a
  separate ~100 MB test program; an `xtask` test fails on a second one).
- **Scope creep rule:** if you notice something else worth doing (a bug,
  refactor, missing feature), do NOT do it. Create a new ticket with the
  `write-ticket` skill and mention it in the PR description.
- **Adding a dependency or third-party asset** (crate, font, JS file, image,
  sound, SDK): check its license against ADR-0013 *before* using it. Only the
  allowed permissive licenses; no GPL/LGPL/MPL/copyleft, no non-commercial, no
  "GPL or buy a commercial license", nothing requiring payment. For crates,
  `cargo deny check licenses` must pass. For anything else, add a row to
  `THIRD_PARTY_ASSETS.md` and commit its license text next to it. If unsure,
  don't add it: pick another or write the small piece yourself.
- A new architectural choice (new dependency with wide impact, new pattern,
  new file format) needs an ADR in the same PR (`write-adr` skill). Adding a
  small, well-known crate for a local need does not.

## 4. Verify

Run the `run-gates` skill. Everything must pass locally before you push.
Walk the ticket's **Acceptance criteria** and tick each box honestly. If one
cannot be met, say so in the Completion notes and in the PR; don't hide it.

## 5. Archive the ticket (same PR)

1. Fill in the ticket's `## Completion notes`: what was done, deviations from the
   plan, follow-up tickets created, anything Nick should know when playing.
   List every **gameplay-affecting rule you had to decide** where the design
   docs were silent (mark them *Claude's starting rule*), in plain words.
   Repeat that list in the PR body and in your final message to Nick, so he
   can agree or veto (Nick asked for this).
2. Set `status: done` and `completed: YYYY-MM-DD` in frontmatter.
3. `git mv tickets/open/<file>.md tickets/done/<file>.md`

## 6. Commit and PR

- Commit messages: `[NNNN] imperative summary`.
- Push and open a PR:
  - Title: `[NNNN] <ticket title>`
  - Body: summary bullets, list of acceptance criteria (checked), follow-up
    tickets created, and `Nick input:` line (e.g. "Sign-off: please play the
    build from the Pages link and try X").
- Don't run mutation testing locally before pushing: CI runs it on the PR
  (see `run-gates`). Push, open the PR, and watch CI once.
- Wait for CI. If a check fails, fix it on the same branch. Never disable a
  gate, lower a threshold, or add `#[mutants::skip]`/`#[allow]` just to pass —
  if a gate is genuinely wrong, write a ticket about it and explain in the PR.
- A PR can merge when its checks are green and GitHub reports no conflict; it
  need not contain the latest `main` (ADR-0042). Merge `main` into the branch
  (`git fetch origin && git merge origin/main`, or the ccd_host
  `sync_with_base_branch` tool in an app worktree) only for a conflict or when
  the PR needs something that landed on `main`.

## Don'ts

- Don't work two tickets in one PR.
- Don't ask Nick technical questions. Decide, and write an ADR if it matters.
- Don't invent game-design answers (stats, magic, story) — those come from
  `docs/design/` and `docs/story/beats.md`.
- Don't edit accepted ADRs; supersede them.

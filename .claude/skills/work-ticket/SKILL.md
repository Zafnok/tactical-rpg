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

## 2. Start

```bash
git switch main && git pull --ff-only
git switch -c t<NNNN>-<slug>
```

Set `status: in-progress` in the ticket frontmatter.

## 3. Implement

- Follow the ticket's **Implementation steps** in order. If a step turns out to
  be wrong, deviate minimally and explain it in the Completion notes.
- Respect crate boundaries (ADR-0004): no I/O, clock or macroquad outside `app`;
  rules only in `core`; state changes only via `Command`s.
- Write the tests the ticket lists (ADR-0007): unit + property for `core`,
  snapshot + scripted integration for screens.
- **Scope creep rule:** if you notice something else worth doing (a bug,
  refactor, missing feature), do NOT do it. Create a new ticket with the
  `write-ticket` skill and mention it in the PR description.
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
2. Set `status: done` and `completed: YYYY-MM-DD` in frontmatter.
3. `git mv tickets/open/<file>.md tickets/done/<file>.md`

## 6. Commit and PR

- Commit messages: `[NNNN] imperative summary`.
- Push and open a PR:
  - Title: `[NNNN] <ticket title>`
  - Body: summary bullets, list of acceptance criteria (checked), follow-up
    tickets created, and `Nick input:` line (e.g. "Sign-off: please play the
    build from the Pages link and try X").
- Wait for CI. If a check fails, fix it on the same branch. Never disable a
  gate, lower a threshold, or add `#[mutants::skip]`/`#[allow]` just to pass —
  if a gate is genuinely wrong, write a ticket about it and explain in the PR.

## Don'ts

- Don't work two tickets in one PR.
- Don't ask Nick technical questions. Decide, and write an ADR if it matters.
- Don't invent game-design answers (stats, magic, story) — those come from
  `docs/design/` and `docs/story/beats.md`.
- Don't edit accepted ADRs; supersede them.

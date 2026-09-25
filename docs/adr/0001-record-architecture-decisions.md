# ADR-0001: Record architecture decisions

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

Nick hands technical control of this project to Claude and will not review
code. Work is done ticket by ticket across many separate model sessions, often
by different models. Sessions have no memory of each other, so a decision that
is not written down will be re-litigated or silently contradicted.

## Decision

- Technical decisions are recorded as ADRs in `docs/adr/`, numbered
  sequentially, using [`0000-template.md`](0000-template.md).
- Game-design decisions (anything Nick is consulted on) are recorded in
  `docs/design/` by the `00xx` decision tickets, not as ADRs.
- A ticket that makes a new architectural choice must add an ADR in the same PR.
- Accepted ADRs are immutable; changes happen by superseding.

## Consequences

Every session can read `docs/adr/` to learn the rules of the codebase in a few
minutes. `CLAUDE.md` points here. The cost is a little writing per decision.

## Alternatives considered

- **Decisions in PR descriptions only** — invisible to future sessions.
- **One big architecture doc** — drifts and gets rewritten, losing the "why".

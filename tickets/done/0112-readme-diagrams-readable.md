---
id: "0112"
title: Make the README's diagrams readable
type: bug
milestone: M0 Foundation
model: sonnet-5
effort: low
status: done
blocked_by: ["0111"]
nick_input: none
completed: 2026-09-30
---

# 0112 — Make the README's diagrams readable

## Context

Ticket 0111 added two Mermaid diagrams to `README.md` (Roadmap and How it's
built). Both were left-to-right chains 6–8 boxes wide, so GitHub scaled them
down until the text was unreadable, and the last box (Steam) sat under
GitHub's zoom controls. Nick reported "our roadmap mermaid didn't render
correctly".

## Nick input

None.

## Scope

**In:** switch both diagrams to top-down layout with shorter labels.

**Out (do not do):** any other README content changes.

## Implementation steps

1. Change `flowchart LR` to `flowchart TD` in both diagrams; shorten labels.
2. Check the rendered README on the branch page on GitHub.

## Acceptance criteria

- [x] Both diagrams render at a readable size on GitHub; no box is hidden.
- [x] ticket-lint and typos pass.

## Tests required

- None (docs only).

## Completion notes

- Both diagrams now flow top to bottom, at most two boxes wide, so GitHub
  renders them near full size. Content is unchanged.

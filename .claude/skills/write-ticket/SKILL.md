---
name: write-ticket
description: Create a new ticket file in tickets/open/ using the repo template — for follow-ups found while working, bugs or feedback Nick reports after playing, or new features. Use whenever work is discovered that is outside the current ticket's scope, or when Nick reports a bug/concern.
---

# Write a ticket

Tickets must be good enough that a weaker model (Sonnet) can complete them
without asking questions.

## Steps

1. **Pick the block** (see `tickets/README.md`): the hundreds digit is the
   milestone/system the work belongs to. Bugs go in the block of the system
   they're in (a combat bug → `03xx`). Post–Chapter 1 features → `10xx`+.
2. **Pick the number:** the next unused number in that block across *both*
   `tickets/open/` and `tickets/done/`:
   ```bash
   ls tickets/open tickets/done | grep -E '^03[0-9]{2}-' | sort | tail -1
   ```
3. Copy `tickets/TEMPLATE.md` to `tickets/open/NNNN-short-kebab-title.md`.
4. Fill in **every** section:
   - **Context:** why this exists; link ADRs, design docs, the ticket/PR that
     discovered it.
   - **Nick input:** `None.` / *Answer first* (name the `00xx` ticket) /
     *Setup* (exact clicks) / *Sign-off* (what to look at).
   - **Scope — in / out:** be explicit about what NOT to do.
   - **Implementation steps:** numbered, concrete, naming files, types and
     functions. Assume the reader knows Rust but not this codebase.
   - **Acceptance criteria:** checkboxes, each objectively verifiable (a test
     name, a command's output, a visible behaviour).
   - **Tests required:** which layers from ADR-0007.
   - Model/effort per the routing table in ADR-0010.
   - `blocked_by`: every ticket whose output this one needs.
5. Keep it small: one PR's worth of work (roughly ≤ 600 changed lines excluding
   snapshots/data). Split otherwise.
6. **Check the chain** (see below) before you finish.

## Check the dependency chain

A ticket is picked up when every id in `blocked_by` is in `tickets/done/`,
so a missing id means a session starts work it can't finish. For every
ticket you write or change:

- **Each open ticket the text names** is one of three things, and the ticket
  says which:
  - *needed first* (its files, types, data, decision or bought assets are
    used) → it is in `blocked_by`;
  - *either order works* → the text says what to do in each case ("if 0805
    isn't done, keep the value in `Ctx` and add a line to 0805");
  - *not this ticket's job* → it is under **Out**.
- **Look the other way too.** Search `tickets/open/` for tickets that need
  what the new one builds or decides, or that the new one makes wrong
  (`grep -rn "<id or feature name>" tickets/open docs/ROADMAP.md`), and
  update their `blocked_by` and text in the same PR.
- **Nothing waits on nothing.** If a ticket waits for a decision, a purchase
  or a setup step, a ticket for that exists and is in `blocked_by`. Use
  `status: blocked` only together with a `blocked_by` id or a written
  reason that names who unblocks it.
- **"Before X" and "after X" are dependencies.** "Do this before the Steam
  page" means X's ticket is blocked by this one; "after the playtest" means
  this one is blocked by 0804.
- If the ticket is needed for Nick's Chapter 1 playtest, add it to 0804's
  `blocked_by` and to the critical path in `docs/ROADMAP.md`. If you can't
  tell whether Nick wants it before the playtest, ask him.
- Run `cargo xtask ticket-lint`.

## Turning Nick's playtest feedback into tickets

Nick reports bugs and concerns in plain language after playing. For each item:

- **Bug** (something is wrong): `type: bug`. Include exact repro steps (keys
  pressed, which map/turn), expected vs actual, and require a failing test that
  reproduces it before the fix. If the report is ambiguous, ask Nick *one*
  concrete question ("Did this happen after the unit was attacked, or before?").
- **Feel/balance** ("cursor feels slow", "enemies too strong"): `type: tuning`.
  Name the data values to change and propose new values.
- **Design change** ("I want magic to work differently"): write a `00xx`
  decision ticket first, then implementation tickets blocked by it.

Reply to Nick with the list of ticket numbers and one-line titles created.

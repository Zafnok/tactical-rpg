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

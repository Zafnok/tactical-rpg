---
id: "0113"
title: Smaller dev builds and cleanup of merged worktrees' build folders
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0113 — Smaller dev builds and cleanup of merged worktrees' build folders

## Context

Each ticket session runs in its own git worktree under `.claude/worktrees/`
(`tickets/README.md`, *Picking the next ticket*), and each worktree has its
own `target/`. On 2026-10-01 those folders held about 315 GB across 60
worktrees; 258 GB of it sat in 51 worktrees whose PR had already merged.
Measured on one worktree (`t0801-game-flow`, 12.5 GB of `target/`):

- `target/debug/deps`: 5.6 GB, of which 5.1 GB is 127 `.exe` files. A
  `cargo test --workspace` links about 20 programs (the game, `xtask`, one
  per `tests/*.rs` file, one per crate's unit tests). On the GNU host
  toolchain (`x86_64-pc-windows-gnu`) the debug info is inside the `.exe`,
  so each is 45–190 MB.
- `target/debug/incremental`: 5.3 GB in 89 folders.
- Cargo never deletes old generations of either, and nothing deletes a
  worktree's `target/` after its PR merges.

This ticket makes each build smaller and removes the build folders nobody
needs. Ticket 0114 cuts the number of test programs.

A shared `target/` for all worktrees was considered and rejected: several
sessions build at once and Cargo holds one lock per build folder, so they
would wait on each other; and the workspace crates still get one copy per
worktree path, so only third-party dependencies would be shared.

## Nick input

`None.`

## Scope

**In:**
- Less debug info in the `dev` and `test` profiles.
- A `cargo xtask clean-merged-targets` command.
- A step in the `work-ticket` skill that runs it.

**Out (do not do):**
- Don't touch `[profile.release]`, CI workflows, or `.cargo/config.toml`'s
  linker settings.
- Don't set a shared `CARGO_TARGET_DIR` / `build.target-dir` (see Context).
- Don't turn off incremental compilation.
- Don't merge test files into fewer programs: ticket 0114.
- The command never removes a worktree, a branch, or anything outside a
  `target/` folder. Claude chat sessions are tied to their worktree folder and
  Nick wants to keep them.

## Implementation steps

1. **Baseline.** On `main`, in a fresh worktree, run `cargo test --workspace
   --no-run` and record the total size of `target/debug/deps/*.exe` and of
   `target/`.
2. **Profiles**, in the root `Cargo.toml`:
   ```toml
   [profile.dev]
   debug = "line-tables-only"   # panics and backtraces keep file:line

   [profile.dev.package."*"]
   opt-level = 2                # existing line, keep
   debug = false                # no debug info for third-party crates
   ```
   `test` inherits `dev`. Add a comment that says why (this ticket's number
   and the sizes). Repeat step 1 and record the new sizes.
3. Check a panic still names its file and line: run any failing assertion
   locally (don't commit it) and read the message.
4. **`cargo xtask clean-merged-targets`** in a new
   `crates/xtask/src/clean_targets.rs`, wired into the `match` in
   `crates/xtask/src/main.rs` like `check-keys`:
   - `git worktree list --porcelain` → (worktree path, branch) pairs.
   - `gh pr list --state all --limit 1000 --json headRefName,state` → PR
     states per branch. Parse with a small hand-written reader or
     `serde_norway` (already a dependency; JSON is valid YAML). No new crates.
   - A worktree's `target/` is deleted only when all of these hold: its
     branch has a `MERGED` PR and no `OPEN` PR; the worktree is not the one
     the command runs in and is not the main checkout; `target/` is a real
     folder (not a link); no file in it was modified in the last 30 minutes
     (another session may be building there).
   - Print one line per worktree: deleted (with GB), or kept and why. Print
     the total freed.
   - `--dry-run` prints the same lines and deletes nothing.
   - If `gh` or `git` is missing or fails, print the error and exit non-zero
     without deleting anything.
   - Keep the decision in a pure function (`fn verdict(pr_states, is_self,
     newest_write_age) -> Verdict`) so it can be unit tested without `gh`.
5. **`work-ticket` skill**, section *2. Start*: after the branch is created,
   add `cargo xtask clean-merged-targets` with one line saying what it does
   and that a failure of this command never blocks the ticket.
6. Add the command to the `xtask` usage text and to the *Map of the repo* /
   *Environment* part of `CLAUDE.md` in one line.

## Acceptance criteria

- [ ] After `cargo test --workspace --no-run` in a fresh worktree, the total
      size of `target/debug/deps/*.exe` is at most half the baseline from
      step 1. Both numbers are in the Completion notes.
- [ ] A panic in a test still prints `file:line`.
- [ ] `cargo xtask clean-merged-targets --dry-run` lists every worktree with
      a verdict and deletes nothing.
- [ ] `cargo xtask clean-merged-targets` deletes `target/` only in worktrees
      whose branch has a merged PR and no open PR, and never the current
      worktree's.
- [ ] Unit tests cover `verdict`: merged → delete; open → keep; no PR →
      keep; merged and open → keep; merged but written 5 minutes ago → keep;
      merged but it is the current worktree → keep.
- [ ] The `work-ticket` skill runs the command in *2. Start*.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `verdict` cases above; parsing of a sample `git worktree list
  --porcelain` output and a sample `gh pr list` JSON.
- Property: none.
- Snapshot / integration: none (the command needs `gh` and real worktrees;
  check it by hand with `--dry-run` and say so in the Completion notes).

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*

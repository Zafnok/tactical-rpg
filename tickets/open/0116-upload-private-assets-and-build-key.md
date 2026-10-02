---
id: "0116"
title: Upload the private assets repo, add its build key, and check the builds use it
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0110"]
nick_input: setup
completed:
---

# 0116 — Upload the private assets repo and add its build key

## Context

Ticket 0110 built everything for the bought art (ADR-0040): the
`private-assets` feature, `cargo xtask private-assets`, the pin file
`assets-private.rev`, and the Pages and release workflows that fetch the art
with the secret `PRIVATE_ASSETS_KEY`. It also arranged the bought bundle on
Nick's machine as a git repository, `D:\tactical-rpg\assets-private\`, with
one commit (`7f960cb6e3c059aa0d98303c36fa463bb360e3b0`, the commit
`assets-private.rev` names).

Two things only Nick can do were left, because they act on his GitHub
account: creating the private repository with that commit in it, and adding
the read-only key. Until then:

- `cargo xtask private-assets` fails in every worktree ("Repository not
  found"), so the tickets that read the bought files (0711, 0436 and what
  follows them) can't be worked in a worktree.
- The Pages build warns and shows placeholders; a real release fails.

## Nick input

**Setup.** Two commands, run once, in this order. Each can be pasted into a
terminal as it is.

1. Create the private repository and upload the files (about 370 MB; it
   takes a few minutes):

   ```bash
   gh repo create Zafnok/visions-of-shuyi-assets --private --description "Bought art for Visions of Shuyi. Must stay private." --source "D:/tactical-rpg/assets-private" --remote origin --push
   ```

2. Make the read-only key the builds use, and give it to both repositories
   (the key files are deleted again at the end; nothing else needs them):

   ```bash
   ssh-keygen -q -t ed25519 -N "" -C "visions-of-shuyi builds" -f "$HOME/shuyi-assets-key" && gh repo deploy-key add "$HOME/shuyi-assets-key.pub" --repo Zafnok/visions-of-shuyi-assets --title "visions-of-shuyi builds (read-only)" && gh secret set PRIVATE_ASSETS_KEY --repo Zafnok/visions-of-shuyi < "$HOME/shuyi-assets-key" && rm "$HOME/shuyi-assets-key" "$HOME/shuyi-assets-key.pub"
   ```

Then tell Claude it is done. Don't change anything in
`D:\tactical-rpg\assets-private\` before the first command: the upload must
contain the commit named above.

## Scope

**In:**
- Checking, after Nick's two commands, that the private repository, the key
  and the builds work as ADR-0040 says, and fixing what doesn't.

**Out (do not do):**
- Creating the repository, the deploy key or the secret for Nick. Claude
  doesn't change access settings on his account.
- Importing any art into `assets-private/game/` (0711, 0436, 0437, 0413).

## Implementation steps

1. `gh repo view Zafnok/visions-of-shuyi-assets --json isPrivate,defaultBranchRef`
   says private, default branch `main`. `git ls-remote
   https://github.com/Zafnok/visions-of-shuyi-assets.git main` prints the
   commit in `assets-private.rev`. If it prints another commit (the local
   repository was changed before the upload), run `cargo xtask
   private-assets --pin` in `D:\tactical-rpg` and commit
   `assets-private.rev` in this ticket's PR.
2. `gh repo deploy-key list --repo Zafnok/visions-of-shuyi-assets` shows one
   key, read-only. `gh secret list --repo Zafnok/visions-of-shuyi` shows
   `PRIVATE_ASSETS_KEY`.
3. In this ticket's worktree (which has no `assets-private/`): `cargo xtask
   private-assets` succeeds and leaves only `README.md`, `.gitignore`,
   `.gitattributes` and `game/` there. `cargo xtask private-assets --library`
   adds `library/tiny-tales/`. `git status` in the worktree shows nothing
   under `assets-private/`.
4. `cargo test -p trpg-content --features private-assets --test
   private_assets` passes, and `cargo build -p trpg-app --features
   private-assets` builds.
5. Run the release workflow as a dry run on `main` (`gh workflow run
   release.yml --ref main`, then `gh run watch`). In each of the four build
   jobs the log of the *Decide whether this run has the bought art* step
   says `Private assets: checking out <the pinned commit>`, the *Check out
   the bought art* step ran, and there is no "building with the public
   placeholders" warning.
6. Run the Pages workflow on `main` (`gh workflow run pages.yml --ref
   main`). Its build job shows the same, plus the *Check the content loads
   with the bought art* step passing.
7. If a step fails because of the workflow or the action, fix it in this
   ticket's PR. If nothing needed fixing, the PR only moves this ticket to
   `done/` with the run links in the Completion notes.

## Acceptance criteria

- [ ] `Zafnok/visions-of-shuyi-assets` is private and its `main` holds the
      commit named in `assets-private.rev`.
- [ ] `cargo xtask private-assets` and `--library` work in a worktree that
      had no `assets-private/`.
- [ ] A release dry run and a Pages run on `main` check out the bought art
      at the pinned commit (run links in the Completion notes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None of its own: this ticket runs what 0110 built. A fix to the workflow
  or the xtask command comes with the test that would have caught it.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*

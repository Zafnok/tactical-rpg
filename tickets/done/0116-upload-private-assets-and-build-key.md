---
id: "0116"
title: Upload the private assets repo, add its build key, and check the builds use it
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: ["0110"]
nick_input: setup
completed: 2026-10-02
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

**Setup.** Two commands, run once, in this order, in a **Command Prompt**
window (`cmd`), which is what Nick uses.

1. ~~Create the private repository and upload the files.~~ **Done
   2026-10-02:** the repository is private and its `main` is the commit
   named above.

   ```bat
   gh repo create Zafnok/visions-of-shuyi-assets --private --description "Bought art for Visions of Shuyi. Must stay private." --source "D:/tactical-rpg/assets-private" --remote origin --push
   ```

2. ~~Make the read-only key the builds use, and give it to both
   repositories.~~ **Done 2026-10-02** (the key files are deleted again at
   the end; nothing else needs them):

   ```bat
   ssh-keygen -q -t ed25519 -N "" -C "visions-of-shuyi builds" -f "%USERPROFILE%\shuyi-assets-key" && gh repo deploy-key add "%USERPROFILE%\shuyi-assets-key.pub" --repo Zafnok/visions-of-shuyi-assets --title "visions-of-shuyi builds (read-only)" && gh secret set PRIVATE_ASSETS_KEY --repo Zafnok/visions-of-shuyi < "%USERPROFILE%\shuyi-assets-key" && del "%USERPROFILE%\shuyi-assets-key" "%USERPROFILE%\shuyi-assets-key.pub"
   ```

   (The first version of this command was written for Git Bash, with
   `$HOME` and `rm`, and failed in Command Prompt before making anything.
   In Git Bash, put `$HOME/` for `%USERPROFILE%\` and `rm` for `del`.)

Then tell Claude it is done.

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

- [x] `Zafnok/visions-of-shuyi-assets` is private and its `main` holds the
      commit named in `assets-private.rev`.
- [x] `cargo xtask private-assets` and `--library` work in a worktree that
      had no `assets-private/`.
- [x] A release dry run and a Pages run on `main` check out the bought art
      at the pinned commit (run links in the Completion notes). *Both ran
      on this ticket's branch, not on `main`: Nick asked for this PR to be
      stacked on 0110's, which wasn't merged yet. See Completion notes.*
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None of its own: this ticket runs what 0110 built. A fix to the workflow
  or the xtask command comes with the test that would have caught it.

## Completion notes

Nick ran both commands on 2026-10-02. Everything worked; nothing in the
workflows, the action or the xtask command needed fixing, so this PR only
closes the ticket.

**Checked:**

1. `Zafnok/visions-of-shuyi-assets` is private, default branch `main`, and
   `git ls-remote` gives `7f960cb6e3c059aa0d98303c36fa463bb360e3b0`, the
   commit in `assets-private.rev`.
2. It has one deploy key, "visions-of-shuyi builds (read-only)", read-only.
   `Zafnok/visions-of-shuyi` has the secret `PRIVATE_ASSETS_KEY`. Both key
   files are gone from Nick's user folder.
3. In the worktree, after removing the `assets-private/` that 0110 had
   cloned from the local copy: `cargo xtask private-assets` fetched from
   GitHub and left `README.md`, `.gitignore`, `.gitattributes` and `game/`;
   `--library` added `library/tiny-tales/` (a hero's face opens as a PNG).
   `git status` showed nothing under `assets-private/`.
4. `cargo test -p trpg-content --features private-assets --test
   private_assets` passed and `cargo build -p trpg-app --features
   private-assets` built.
5. **Release dry run, with the key**
   (<https://github.com/Zafnok/visions-of-shuyi/actions/runs/37072823623>):
   success. Each of `build-windows`, `build-linux`, `build-macos` and
   `build-web` printed `Private assets: checking out 7f960cb6…`, checked
   out only `game/`, and built with the feature (`--features
   private-assets`, or `--private-assets` for web). No placeholder warning.
6. **Pages run, with the key**
   (<https://github.com/Zafnok/visions-of-shuyi/actions/runs/37072826249>):
   the `build` job checked out the same commit, the *Check the content
   loads with the bought art* step passed, and `cargo xtask web --release
   --debug-tools --private-assets` built. The run is marked failed because
   its `deploy` job was refused ("Branch … is not allowed to deploy to
   github-pages due to environment protection rules"): the Pages
   environment only takes `main`. Nothing was published.

**Also seen:** a release dry run started before the key existed
(<https://github.com/Zafnok/visions-of-shuyi/actions/runs/37072359624>)
took the other path on all four platforms: the "building with the public
placeholders" warning, and a successful build without the feature.

**Deviations:**

- Steps 5 and 6 ran on this ticket's branch instead of `main`: Nick asked
  for this PR to be stacked on 0110's (#165) rather than wait for its
  merge. The workflows and the action are the same files. What wasn't
  seen is a Pages **deploy** from `main` with the art in it; that happens
  on the first merge to `main` after #165. There is nothing to look at in
  the game yet either way: `assets-private/game/` is empty until 0711 and
  0436 import art.
- The key command in this ticket was first written for Git Bash and failed
  in Nick's Command Prompt before making anything; the ticket now has the
  Command Prompt version.

**Follow-up tickets:** none.

**Gameplay rules decided:** none.

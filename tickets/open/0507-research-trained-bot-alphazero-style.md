---
id: "0507"
title: "Research: a trained AlphaZero-style bot (stack ADR + prototype)"
type: research
milestone: M4 Enemy AI
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["0506"]
nick_input: none
completed:
---

# 0507 — Research: a trained AlphaZero-style bot

## Context

Nick's original idea (2026-09-30) was AlphaZero-style: give an AI the controls
and the win condition, not a strategy, and let it learn. 0506's search bots
give the player types without training. A trained bot adds two things:
- A stronger player than search alone, to stand in for the best Hardcore
  players.
- An **exploit finder**: learning agents are good at discovering degenerate
  strategies (an unbeatable chokepoint, a skill combo that trivialises a map)
  that a designer and a scripted bot won't try.

Decided in the 2026-09-30 conversation, to carry into this ticket:
- **Training checkpoints are not player types.** A half-trained network is
  weak in inhuman ways, and "epoch N" means nothing across maps or network
  sizes. Skill is measured by the 0505 report (win rate, turns, fallen,
  items), never by training time.
- **No self-play.** The enemy is the fixed scripted AI (`ai.rs`), so a battle
  is a one-player game with dice. The fitting AlphaZero relative is
  **Stochastic MuZero** (Antonoglou et al., 2022: MuZero with chance nodes
  for dice), or AlphaZero-style MCTS with chance handled by sampling reseeded
  copies (0504). Policy-gradient **PPO with invalid-action masking**
  (Huang & Ontañón, 2020) is the simpler baseline.
- **Not a chatbot model.** No pre-trained model plays FE-likes. A small
  network of our own (a few MB) is the plan.
- Bots never rewind (Nick).
- Hardware for local training: Nick's laptop has an NVIDIA RTX 5090 Laptop
  GPU, an Intel Core Ultra 9 275HX and 64 GB RAM. Blackwell GPUs need a
  CUDA 12.8+ build of PyTorch (2.7 or later) if Python is chosen.

## Nick input

None. Installing tools on Nick's machine (Python, WSL2, CUDA wheels) is done
by the session, with Nick's OK for each install.

## Scope

**In:**
- An ADR choosing the stack, after a short spike of the top two options.
- Observation and action encoding of a battle for a network.
- A prototype that trains on a small battle and is evaluated by the 0505
  runner as a `PlayerBot`.
- A written report with numbers and follow-up tickets.

**Out (do not do):**
- Shipping a trained model or using one for the enemy AI. That would need
  its own ADR and Nick's design call.
- Training on every battle or a CI training job.
- Changing `core` beyond small bot-only helpers (each named in the ADR).

## Implementation steps

1. **Spike the stack** (≤ 1 day each), candidates:
   - **Python + PyTorch** with **LightZero** (OpenDILab; AlphaZero, MuZero,
     Stochastic MuZero, Gumbel MuZero implementations) or **sb3-contrib
     MaskablePPO**, talking to Rust through a **PyO3/maturin** extension
     crate (`crates/bots-py`, not a workspace default member). Risk to check
     first: Windows Python is MSVC-built and the local toolchain is GNU
     (CLAUDE.md "Environment"); try PyO3's `generate-import-lib` on
     windows-gnu, and running training under **WSL2 Ubuntu** (CUDA works
     there) as the fallback.
   - **Pure Rust** with **`burn`** (MIT/Apache-2.0, CUDA/wgpu backends):
     no language boundary, more to write (no ready AlphaZero/MuZero).
   Check each dependency's license and maintenance status at the time;
   record versions.
2. **ADR** (`write-adr`): the stack, where the code lives (e.g. `tools/trainer/`
   outside the cargo workspace for Python), how it builds on Nick's machine,
   and that none of it ships (ADR-0013 covers shipped code only).
3. **Encoding** (documented in `docs/playtesting.md`):
   - Observation: fixed-size planes over the map (terrain id / move cost,
     each unit's faction, HP %, stats, acted flag, weapon range, danger zone
     from `trpg_core::danger_zone`) padded to a max map size, plus globals (turn,
     phase, objective).
   - Action: factorised as unit → destination tile → action (wait, attack
     target+slot, cast, item, seize, …), with masks built from
     `legal_commands` (0504); anything the mask can't express falls back to
     an index into `legal_commands`' list.
4. **Environment API** on the Rust side: `reset(battle, seed)`,
   `legal_mask()`, `step(action) -> (obs, reward, done)`; the enemy phase
   runs inside `step` with `ai::next_command`. Reward: +1 win, −1 loss, and
   the Hardcore persona's scoring (0506) as optional shaping.
5. **Prototype run** on `test_small` (or the smallest battle file), then
   Chapter 1 if time allows. Record: environment steps per second, training
   wall time, the 0505 report for the trained bot every N updates, and a
   comparison with 0506's Hardcore bot on the same seeds.
6. **Report** in Completion notes and `docs/playtesting.md`: does the trained
   bot beat Hardcore? What did it learn that the search bots didn't? Any
   exploits found become `write-ticket` tickets (`tuning` or `bug`).
   Recommend go / no-go for a follow-up that productionises it.

## Acceptance criteria

- [ ] ADR merged with the chosen stack and the spike's findings for both
      candidates (what worked, what didn't, timings).
- [ ] The prototype trains from a clean checkout following documented steps
      in `docs/playtesting.md`.
- [ ] The trained bot runs through `cargo xtask playtest` (or an equivalent
      documented command) and its report is in Completion notes next to the
      Hardcore bot's on the same seeds.
- [ ] Follow-up tickets are filed for exploits found and for the go/no-go
      recommendation.
- [ ] Rust code added stays within one PR's size (≈ 600 lines); Python
      prototype code lives under `tools/trainer/` with its own README.
- [ ] All gates in the `run-gates` skill pass (the Python tooling is outside
      the gates unless the ADR adds it).

## Tests required

- Unit: the Rust environment API (`reset`/`step`/mask agree with
  `legal_commands`; same seed + same actions → same observations).
- Property: every masked-legal action is accepted by `BattleState::apply`.
- Snapshot / integration: none required for the research prototype.

## Completion notes


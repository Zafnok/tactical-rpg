# ADR-0048: Player settings: one saved RON record, read through `Ctx`

- **Status:** Accepted
- **Date:** 2026-10-03
- **Related tickets:** 0805, 0208, 0217, 0416, 0806, 0824, 0825, 0826, 0902

## Context

Until the Options menu (0805) each player preference lived wherever its
ticket left it: the layout under its own `Storage` key (`layout`, 0208),
the cursor style and the text speed as fields of `Ctx`, auto-end as a field
of the battle screen that every battle reset. None but the layout was
saved. Options adds more (animation and enemy-phase speed, combat
animations, fullscreen, music and sound volume), and later tickets add a
colour theme (0806), the map look (0824), the language (0825) and voices
(0826).

Two of them can't be applied inside `ui` (ADR-0004): only `app` can make
the window fullscreen or scale what it plays.

## Decision

- **One struct, `trpg_ui::settings::Settings`**, holds every preference.
  Its `Default` is how the game behaved before it had options.
- **Saved as one RON record under the `Storage` key `settings`**
  (`Settings::to_ron` / `from_ron`), through a private mirror struct with a
  `version` (1) and `#[serde(default)]`: **a field missing from the saved
  text takes its default**, so a ticket that adds a setting adds a field
  with a default and bumps nothing. Text that can't be read, or of another
  version, gives the defaults and a warning for `app` to log (as the key
  bindings do, ADR-0031). Out-of-range values are repaired on load (a
  volume over 10, an unknown layout name).
- **`Ctx` owns the settings.** `Ctx::with_storage` loads them;
  `ctx.settings()` reads them; `ctx.change_settings(|s| …)` edits them and
  **saves at once** if anything changed. A change applies even when saving
  fails. There is no "apply" step: consumers read `ctx.settings()` when
  they need a value (the dialogue typewriter, the battle screen at the
  start of each frame), so a change made on the Options screen over a
  battle holds from the next frame.
- **The layout is in the settings but changes only through
  `Ctx::choose_layout`**, which also switches the keys;
  `change_settings` can't alter it. The old `layout` key is still read
  when the settings have no layout (a player from before this ADR), and
  never written.
- **Key bindings stay apart** under `keybindings` (ADR-0031): they are per
  layout, repaired differently, and edited on their own screen.
- **What only `app` can do goes out in `FrameOutput`** every frame, as
  state, not as an event: `fullscreen: bool`, `music_volume` and
  `sound_volume` (0–1 multipliers on each cue's own volume, ADR-0026).
  `app` compares with what it last applied, so the saved values take hold
  on the first frame of a launch with no separate start-up path. The test
  `Harness` records them (`h.fullscreen()`, `h.volumes()`).
- **Animation speeds scale the frame time** the battle screen hands its
  walk, fight playback, AI action and EXP bar clocks
  (`Settings::battle_speed`), on top of the hold-Confirm speed-up. Banners,
  toasts and the cursor's pulse keep real time. `core` is untouched: the
  battle is the same at any speed (a test checks it).
- **The campaign's mode is not a setting** (it is part of the save,
  `death-and-difficulty.md`). The Options screen shows and switches it
  through `Ctx::campaign_mode`, which the game flow (ADR-0035) keeps in
  step with its `Campaign`.

## Consequences

- Adding a setting is a field, a default, a row on the Options screen and
  its consumer reading `ctx.settings()`.
- A screen test changes a setting with `ctx.change_settings(…)` and a
  Harness test with `h.ctx_mut().change_settings(…)` or the Options screen's
  keys.
- The settings are written on every change, a few hundred bytes; holding a
  key on a volume row writes once per step.
- A saved value serde can't read (a renamed enum variant) loses every
  setting to the defaults, layout included, so the layout picker would open
  again. Don't rename variants; add new ones.
- `app` learns the fullscreen state only from the setting: a player who
  leaves fullscreen with the browser's own key is out of step until they
  flip the row (0902 adds the window keys).

## Alternatives considered

- **One `Storage` key per setting** (as `layout` was): no format to
  version, but a dozen reads at start-up and a dozen places to forget a
  default.
- **Settings inside the key-bindings config:** one file, but the key
  bindings' repair rules (ADR-0031) would have to cover unrelated values,
  and "Restore defaults" on either screen would need to spare the other's.
- **Requests in `FrameOutput` only when a value changes** (like audio
  requests): `app` would need the starting values some other way, and a
  missed request would leave it wrong for the rest of the run.
- **Per-consumer speed constants picked by the setting** (two `Timings`
  tables): every timing would exist twice and drift; scaling time keeps
  one table.

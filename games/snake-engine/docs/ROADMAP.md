# Roadmap: Snake → SnakeEngine → SnakeED

The stated goal is the Epic Games arc in miniature: ship a game on a small,
solid engine, then let the engine outgrow the game. Unreal Tournament came
years before UnrealEd was a serious level editor, and UnrealEd came years
before Unreal Engine was something other studios licensed. This roadmap is
staged the same way — each version is a real, playable/usable milestone, not
a placeholder.

## v0.1 — Foundation (this delivery)
- Playable cursor-driven snake: grid-locked movement steered continuously by
  the mouse cursor, auto-pickup on contact (no separate "use item" input).
- Data-driven bonus/drawback system: 7 categories, each with an
  algorithmically-rolled polarity and severity tier — from a flat -1 life
  drawback up to a Catastrophic -99%-of-current-life roll — defined in
  `data/effects/core_effects.json`, not hardcoded.
- Persistent meta-progression: essence earned per run, spent on 4 upgrades
  (Vitality, Luck, Insurance, Regeneration) that carry into the next run.
- CMake build that works from a clean checkout on Linux (Ubuntu and Fedora
  both have dedicated `scripts/build_*.sh`), macOS, and Windows, fetching
  SDL2/nlohmann_json from source when no system package is found.
- Android (Gradle + NDK, `android/`) and Web (Emscripten + WebAssembly,
  `scripts/build_web.sh`) builds of the same `engine/` + `game/`, with touch
  and mouse both driving the same cursor-follow steering.
- `engine` (static library) / `game` (executable) split so the editor can
  link the same core later without depending on the game's `main.cpp`.

## v0.2 — Depth
- More effect categories and tiers; item rarity read from `spawn_weight`
  instead of a fixed list.
- Replace the seven-segment HUD with a real text/UI layer.
- Gamepad and touch input (the cursor-follow model generalizes to a stick
  vector or a touch point, not just a mouse).
- Sound and simple particle feedback for pickups, especially severe
  drawbacks — right now they're silent, which undersells "life-ending".
- Save-file versioning/migration so v0.1 saves don't break on upgrade.

## v0.3 — Levels as data
- A level/arena format (JSON, alongside the effect data) describing field
  size, walls, hazards, and spawn zones, loaded instead of the current
  hardcoded 48×32 open grid.
- Multiple built-in arenas ship as data, proving the format before any
  editor exists to author it.

## v0.4 — SnakeED, milestone 1: live tuning
- An in-process debug overlay (Dear ImGui, added as its own optional CMake
  target so `game` doesn't gain a UI-toolkit dependency by default) for
  editing effect tiers and watching the change apply immediately, with
  hot-reload of `data/effects/*.json`. This is the "F8 in UEd" moment —
  editing becomes something you do while the game runs, not a recompile.

## v0.5 — SnakeED, milestone 2: standalone editor
- A separate `editor/` executable, linking `snake_engine` the same way
  `game` does: level layout (place walls/spawns), effect/item authoring,
  and a snake skin/character editor, all writing the same JSON the game
  loads at runtime.

## v0.6 — Environment & modes
- Themeable backgrounds/palettes, alternate arena topologies (walled vs.
  wraparound), and additional game modes built as data + a small amount of
  mode-specific logic, to start proving the engine can host more than one
  game shape.

## v1.0 — Scripting
- An embedded scripting layer (most likely Lua) for game-mode and effect
  logic that doesn't require touching engine C++ — the Blueprints-style
  jump from "recompile to change behavior" to "author behavior as data or
  script".

## v2.0+ — Speculative
- Optional 3D/other-genre rendering backend, networked multiplayer, and an
  asset/mod sharing story. This is the "UE6" end state the project is
  named after chasing — explicitly long-range, and not scoped in detail
  until the v0.x milestones above are real.

Contributions that move any single checkbox above forward are welcome; see
the repository's root `CONTRIBUTING.md`.

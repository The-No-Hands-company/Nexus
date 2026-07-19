# SnakeEngine

A cursor-driven snake game, and the seed of a full 2D level / character /
environment editor (**SnakeED**) built on top of the same engine — the same
arc Epic walked from Unreal Tournament to UnrealEd to Unreal Engine, aimed
at Snake instead. See [`docs/ROADMAP.md`](docs/ROADMAP.md) for the staged
plan and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for how the current
code is put together.

This is a subproject inside the Nexus monorepo, independent of the Rust
chat platform in the rest of the repository — its own CMake build, its own
C++20 codebase, licensed the same as the rest of Nexus
([AGPL-3.0-or-later](../../LICENSE)).

## What makes it a "snake-like" and not classic Snake

- **Movement follows the cursor**, continuously — no arrow keys. The snake
  is still grid-locked and moves one cell per tick, but every tick it turns
  toward wherever the mouse currently is.
- **Auto-pickup.** Touching an item consumes it immediately; there's no
  separate "use" input.
- **Every item is a gamble.** Each one is, at the moment it spawns, rolled
  into a category (Life, Speed, Size, Score, Control, Vision, Shield), a
  polarity (bonus or drawback), and a severity tier — algorithmically, via
  weighted random selection defined in `data/effects/core_effects.json`.
  The player never picks the tier. Life drawbacks range from a flat **-1**
  up to a Catastrophic roll that takes **-99% of your life as of the
  instant you pick it up** — a near-run-ending swing that's rare (weight
  0.5 out of roughly 100) but real.
- **Bonus upgrades persist between runs.** Essence earned from a run's score
  buys permanent upgrades (Vitality, Luck, Insurance, Regeneration) that
  carry into the next attempt.

## Controls

| Input | Effect |
|---|---|
| Mouse movement | Steer the snake |
| `P` / `Space` | Pause / unpause |
| `R` | Restart (after game over) |
| `1` `2` `3` `4` | Buy Vitality / Luck / Insurance / Regeneration (game-over screen only) |
| `Esc` | Quit |

## Building

Requires a C++20 compiler and CMake 3.20+. System SDL2 and nlohmann-json
speed up the build and are picked up automatically if present; otherwise
CMake fetches both from source (`cmake/FetchDeps.cmake`) — either way, a
clean checkout builds with no manual dependency install required.

### Linux
```bash
./scripts/build_linux.sh          # optionally apt-installs SDL2/nlohmann-json first
./build/game/snake_game
```

### macOS
```bash
./scripts/build_macos.sh          # optionally brew-installs SDL2/nlohmann-json first
./build/game/snake_game
```

### Windows
From a "Developer PowerShell for VS":
```powershell
powershell -ExecutionPolicy Bypass -File scripts\build_windows.ps1
.\build\game\RelWithDebInfo\snake_game.exe
```

### Manual (any platform)
```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build --parallel
ctest --test-dir build          # run the unit tests
```

`SNAKE_BUILD_GAME` and `SNAKE_BUILD_TESTS` CMake options (both `ON` by
default) let you build just the `snake_engine` library if you're embedding
it elsewhere.

## Project layout

```
snake-engine/
├── engine/     snake_engine — the reusable core (math, RNG, grid, snake,
│               effect system, item spawner, upgrades, save, SDL2 window
│               + renderer). No game-specific glue.
├── game/       the playable executable — wires engine pieces together
├── editor/     SnakeED, not built yet — see docs/ROADMAP.md
├── data/       JSON-authored effect/item definitions
├── tests/      dependency-free CTest unit tests
├── scripts/    per-platform build helpers
└── cmake/      dependency resolution (system package, else FetchContent)
```

## Status

Early. This is a solid, playable, tested foundation — not yet the editor.
See `docs/ROADMAP.md` for what's actually built versus planned; contributions
that move a roadmap item forward are welcome.

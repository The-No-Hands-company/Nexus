# SnakeEngine — Architecture

## Layout

```
games/snake-engine/
├── engine/            snake_engine — static library, no game-specific glue
│   ├── include/snake_engine/   public headers (math, rng, grid, snake,
│   │                            effect, effect_catalog, item, upgrades,
│   │                            save, game, window, renderer)
│   └── src/                    implementations (one SDL2 backend today)
├── game/              snake_game — the playable executable, wires engine
│                       pieces together in main.cpp
├── editor/             SnakeED — not built yet, see docs/ROADMAP.md
├── data/effects/       JSON effect/item definitions (data, not code)
├── tests/              dependency-free CTest unit tests
├── scripts/            per-platform build helpers
└── cmake/              dependency resolution (FetchDeps.cmake)
```

`engine` depends on SDL2 (window/input/2D rendering) and nlohmann_json (data
loading). Nothing else. `game` depends only on `engine`. This separation is
deliberate: it's the same split SnakeED (the future editor) will sit on top
of — the editor will link `snake_engine` too, and add its own UI/tooling
layer without the engine ever needing to know an editor exists.

## Core loop

`Game` (engine/include/snake_engine/game.hpp) owns one run's worth of state:
the `Snake`, the `ItemSpawner`, life/score/shield counters, and a list of
currently-active timed effects. `game/src/main.cpp` drives it:

1. Poll SDL events into an `InputState` (cursor position, quit/restart/pause).
2. `Game::update(dt, cursorPixelPos)` — steers the snake toward the cursor,
   advances the grid-tick clock, resolves item pickups, ticks down timed
   effects, applies passive regen.
3. `GameRenderer::render(game)` draws the board, snake, items, HUD, and a
   vision-drawback vignette.
4. On game over, essence is banked and a keyboard-driven upgrade shop is
   shown (`GameRenderer::renderGameOverShop`).

## Movement: cursor-follow, not key-press

`Snake::steerToward(Vec2f cursorGridPos)` compares the vector from the
snake's head to the cursor and picks whichever cardinal direction best
matches it, refusing a direct 180° reversal. This runs every frame (not just
on a movement tick), so direction changes feel immediate even though the
snake still advances one grid cell per tick — the tick rate is what
`Speed` effects modify.

## The effect system

This is the part that turns "snake game" into something with actual game
design levers.

- `EffectCategory` (Life, Speed, Size, Score, Control, Vision, Shield) groups
  related bonus/drawback pairs.
- `SeverityTier` is one rung on a category+polarity's ladder: a name, a
  relative spawn weight, a magnitude, an optional duration, and how the
  magnitude should be interpreted (`MagnitudeType`: flat life, percent of
  *current* life, flat score, or a generic scalar).
- `EffectCatalog::rollRandomEffect(rng, luck)` is the single place a
  category, a polarity (bonus vs. drawback), and a severity tier are chosen —
  algorithmically, via weighted random selection, never by the player. It
  runs once per item spawn (`ItemSpawner::update`), so an item's visual
  color/intensity is fixed the moment it appears.
- `Game::applyRolledEffect` is where a rolled effect actually changes the
  player. The one rule worth calling out: **Life-category drawbacks are
  resolved as a percentage of the player's life at the moment of pickup**,
  not of their max life — a Catastrophic roll (-99%, weight 0.5 out of
  ~100) takes 99% of however much life you have *right now*. See
  `resolveLifeDelta` in effect.hpp/.cpp, which is a pure function so this
  rule has a direct unit test (tests/test_effects.cpp) independent of SDL or
  the rest of `Game`.
- Everything above is authored in `data/effects/core_effects.json`, not
  hardcoded. `EffectCatalog::loadBuiltinDefaults()` mirrors the same data as
  a fallback so the game is playable even if the JSON file is missing —  but
  the JSON file is the real source of truth, and it's exactly the shape
  SnakeED's effect editor will read and write.

## Meta-progression

`MetaProgress` (essence balance + `UpgradeLevels`) is separate from a single
run's `Game` state and persists to `snake_engine_save.json` next to the
executable (`save.hpp`/`save.cpp`, plain JSON via nlohmann_json — no
platform-specific "app data directory" API, so it behaves identically on
every OS). `computeModifiers()` turns upgrade levels into the numbers `Game`
actually uses (`PlayerStatModifiers`): bonus max life, a luck value that
reshapes future item rolls, starting shield charges, and passive regen.

## Rendering without a font dependency

`GameRenderer` draws seven-segment digits with plain filled rectangles
instead of pulling in SDL_ttf and a bundled font. That's a deliberate
tradeoff to keep the "clone and build on any OS" story simple — it costs
some visual polish today, and is one of the first things a real UI layer
(arriving with SnakeED, see docs/ROADMAP.md) will replace.

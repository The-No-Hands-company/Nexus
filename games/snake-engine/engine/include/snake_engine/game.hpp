#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "snake_engine/effect_catalog.hpp"
#include "snake_engine/grid.hpp"
#include "snake_engine/item.hpp"
#include "snake_engine/rng.hpp"
#include "snake_engine/snake.hpp"
#include "snake_engine/upgrades.hpp"

namespace snake_engine {

enum class GameStatus { Playing, GameOver };

struct ActiveTimedEffect {
    RolledEffect effect;
    float remainingSeconds;
};

// Owns one run's worth of state: the snake, the field, active items, active
// timed buffs/debuffs, life and score. Everything about "what happens when
// an item is picked up" lives here in applyRolledEffect(), which is the one
// place that turns a RolledEffect into an actual change to the player.
class Game {
public:
    Game(Grid grid, const EffectCatalog& catalog, PlayerStatModifiers mods);

    void reset();
    void update(float dt, Vec2f cursorPixelPos);

    // Applied on the next reset() — used when the player buys an upgrade
    // between runs and then starts a new one.
    void setModifiers(PlayerStatModifiers mods) { mods_ = mods; }

    [[nodiscard]] GameStatus status() const { return status_; }
    [[nodiscard]] const std::string& gameOverReason() const { return gameOverReason_; }
    [[nodiscard]] float life() const { return life_; }
    [[nodiscard]] float maxLife() const { return maxLife_; }
    [[nodiscard]] int64_t score() const { return score_; }
    [[nodiscard]] int shieldCharges() const { return shieldCharges_; }
    [[nodiscard]] const Snake& snake() const { return snake_; }
    [[nodiscard]] const ItemSpawner& itemSpawner() const { return itemSpawner_; }
    [[nodiscard]] const std::vector<ActiveTimedEffect>& activeEffects() const {
        return activeEffects_;
    }
    [[nodiscard]] const Grid& grid() const { return grid_; }

    [[nodiscard]] float speedMultiplier() const;
    [[nodiscard]] float controlNoise() const;
    [[nodiscard]] float visionMultiplier() const;

private:
    Grid grid_;
    PlayerStatModifiers mods_;
    Rng rng_;

    Snake snake_;
    ItemSpawner itemSpawner_;

    GameStatus status_ = GameStatus::Playing;
    std::string gameOverReason_;
    float life_ = 100.0f;
    float maxLife_ = 100.0f;
    int64_t score_ = 0;
    int shieldCharges_ = 0;

    float moveTimer_ = 0.0f;
    float baseMoveIntervalSeconds_ = 0.14f;
    std::vector<ActiveTimedEffect> activeEffects_;

    void applyRolledEffect(const RolledEffect& effect);
    void tickActiveEffects(float dt);
    void endGame(const std::string& reason);
};

}  // namespace snake_engine

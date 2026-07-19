#include "snake_engine/game.hpp"

#include <algorithm>
#include <cmath>

namespace snake_engine {

namespace {
Vec2i startPositionFor(const Grid& grid) {
    return Vec2i{grid.width / 2, grid.height / 2};
}
}  // namespace

Game::Game(Grid grid, const EffectCatalog& catalog, PlayerStatModifiers mods)
    : grid_(grid),
      catalog_(catalog),
      mods_(mods),
      snake_(startPositionFor(grid), 4),
      itemSpawner_(grid.width, grid.height, catalog) {
    reset();
}

void Game::reset() {
    snake_.reset(startPositionFor(grid_), 4);
    status_ = GameStatus::Playing;
    gameOverReason_.clear();
    maxLife_ = 100.0f + mods_.maxLifeBonus;
    life_ = maxLife_;
    score_ = 0;
    shieldCharges_ = mods_.startingShields;
    moveTimer_ = 0.0f;
    activeEffects_.clear();
}

void Game::endGame(const std::string& reason) {
    status_ = GameStatus::GameOver;
    gameOverReason_ = reason;
}

float Game::speedMultiplier() const {
    float multiplier = 1.0f;
    for (const auto& active : activeEffects_) {
        if (active.effect.category == EffectCategory::Speed) {
            multiplier *= static_cast<float>(active.effect.magnitude);
        }
    }
    return multiplier;
}

float Game::controlNoise() const {
    float noise = 0.0f;
    for (const auto& active : activeEffects_) {
        if (active.effect.category == EffectCategory::Control &&
            active.effect.polarity == EffectPolarity::Drawback) {
            noise = std::max(noise, static_cast<float>(active.effect.magnitude));
        }
    }
    return noise;
}

float Game::visionMultiplier() const {
    float multiplier = 1.0f;
    for (const auto& active : activeEffects_) {
        if (active.effect.category == EffectCategory::Vision) {
            multiplier *= static_cast<float>(active.effect.magnitude);
        }
    }
    return multiplier;
}

void Game::tickActiveEffects(float dt) {
    for (auto& active : activeEffects_) {
        active.remainingSeconds -= dt;
    }
    activeEffects_.erase(std::remove_if(activeEffects_.begin(), activeEffects_.end(),
                                         [](const ActiveTimedEffect& a) {
                                             return a.remainingSeconds <= 0.0f;
                                         }),
                          activeEffects_.end());
}

void Game::applyRolledEffect(const RolledEffect& effect) {
    if (effect.polarity == EffectPolarity::Drawback && shieldCharges_ > 0) {
        --shieldCharges_;
        return;  // a Ward/Aegis charge fully absorbs one drawback
    }

    switch (effect.category) {
        case EffectCategory::Life: {
            // Resolved against life AT PICKUP TIME, per design: a -99%
            // Catastrophic drawback takes 99% of whatever life the player
            // has right now, not of their max life.
            float delta = resolveLifeDelta(effect.magnitudeType, effect.magnitude, life_);
            life_ = clamp(life_ + delta, 0.0f, maxLife_);
            if (life_ <= 0.0f) {
                endGame("Life depleted");
            }
            break;
        }
        case EffectCategory::Size: {
            int amount = static_cast<int>(std::round(effect.magnitude));
            if (amount > 0) {
                snake_.grow(amount);
                score_ += amount * 2;
            } else if (amount < 0) {
                if (!snake_.shrink(-amount)) {
                    endGame("Withered away to nothing");
                }
            }
            break;
        }
        case EffectCategory::Score: {
            score_ = std::max<int64_t>(0, score_ + static_cast<int64_t>(effect.magnitude));
            break;
        }
        case EffectCategory::Speed:
        case EffectCategory::Control:
        case EffectCategory::Vision: {
            if (effect.durationSeconds > 0.0f) {
                activeEffects_.push_back(ActiveTimedEffect{effect, effect.durationSeconds});
            }
            break;
        }
        case EffectCategory::Shield: {
            shieldCharges_ += static_cast<int>(std::round(effect.magnitude));
            break;
        }
    }
}

void Game::update(float dt, Vec2f cursorPixelPos) {
    if (status_ != GameStatus::Playing) {
        return;
    }

    tickActiveEffects(dt);

    Vec2f cursorGrid = grid_.pixelsToGridSpace(cursorPixelPos);

    float noise = controlNoise();
    if (noise > 0.0f) {
        cursorGrid.x += (rng_.nextFloat01() * 2.0f - 1.0f) * noise * 3.0f;
        cursorGrid.y += (rng_.nextFloat01() * 2.0f - 1.0f) * noise * 3.0f;
    }

    snake_.steerToward(cursorGrid);

    float moveInterval = baseMoveIntervalSeconds_ / std::max(speedMultiplier(), 0.15f);
    moveTimer_ += dt;
    if (moveTimer_ >= moveInterval) {
        moveTimer_ -= moveInterval;

        bool alive = snake_.tick();
        if (!alive) {
            endGame("Ate its own tail");
            return;
        }
        if (!grid_.inBounds(snake_.head())) {
            endGame("Hit the wall");
            return;
        }

        score_ += 1;  // small trickle for staying alive / covering ground

        Vec2i head = snake_.head();
        const Item* eaten = nullptr;
        for (const auto& item : itemSpawner_.items()) {
            if (item.position == head) {
                eaten = &item;
                break;
            }
        }
        if (eaten != nullptr) {
            RolledEffect effect = eaten->effect;  // copy: spawner erases the item next
            itemSpawner_.removeItemAt(head);
            applyRolledEffect(effect);
        }
    }

    if (status_ == GameStatus::Playing && mods_.regenPerSecond > 0.0f) {
        life_ = std::min(maxLife_, life_ + mods_.regenPerSecond * dt);
    }

    std::vector<Vec2i> occupied(snake_.body().begin(), snake_.body().end());
    itemSpawner_.update(dt, rng_, mods_.luck, occupied);
}

}  // namespace snake_engine

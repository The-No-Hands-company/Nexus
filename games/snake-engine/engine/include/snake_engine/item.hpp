#pragma once

#include <vector>

#include "snake_engine/effect.hpp"
#include "snake_engine/effect_catalog.hpp"
#include "snake_engine/math.hpp"
#include "snake_engine/rng.hpp"

namespace snake_engine {

struct Item {
    Vec2i position;
    RolledEffect effect;
};

// Keeps the grid stocked with items. Every item's effect is rolled at spawn
// time (see EffectCatalog::rollRandomEffect) — the "which category / bonus
// or drawback / how severe" decision is made here, algorithmically, well
// before the player ever sees or touches the item.
class ItemSpawner {
public:
    ItemSpawner(int gridWidth, int gridHeight, const EffectCatalog& catalog);

    void update(float dt, Rng& rng, float luck, const std::vector<Vec2i>& occupiedCells);

    void removeItemAt(const Vec2i& pos);

    [[nodiscard]] const std::vector<Item>& items() const { return items_; }

    void setMaxActiveItems(int max) { maxActiveItems_ = max; }
    void setSpawnIntervalSeconds(float seconds) { spawnIntervalSeconds_ = seconds; }

private:
    int gridWidth_;
    int gridHeight_;
    const EffectCatalog& catalog_;
    std::vector<Item> items_;
    float timeSinceLastSpawn_ = 0.0f;
    float spawnIntervalSeconds_ = 2.0f;
    int maxActiveItems_ = 6;

    [[nodiscard]] bool isCellFree(const Vec2i& pos, const std::vector<Vec2i>& occupiedCells) const;
};

}  // namespace snake_engine

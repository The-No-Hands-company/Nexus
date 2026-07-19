#include "snake_engine/item.hpp"

#include <algorithm>

namespace snake_engine {

ItemSpawner::ItemSpawner(int gridWidth, int gridHeight, const EffectCatalog& catalog)
    : gridWidth_(gridWidth), gridHeight_(gridHeight), catalog_(catalog) {}

bool ItemSpawner::isCellFree(const Vec2i& pos, const std::vector<Vec2i>& occupiedCells) const {
    if (std::any_of(occupiedCells.begin(), occupiedCells.end(),
                     [&](const Vec2i& c) { return c == pos; })) {
        return false;
    }
    return std::none_of(items_.begin(), items_.end(),
                         [&](const Item& item) { return item.position == pos; });
}

void ItemSpawner::update(float dt, Rng& rng, float luck, const std::vector<Vec2i>& occupiedCells) {
    timeSinceLastSpawn_ += dt;
    if (static_cast<int>(items_.size()) >= maxActiveItems_) {
        return;
    }
    if (timeSinceLastSpawn_ < spawnIntervalSeconds_) {
        return;
    }
    timeSinceLastSpawn_ = 0.0f;

    constexpr int kMaxAttempts = 50;
    for (int attempt = 0; attempt < kMaxAttempts; ++attempt) {
        Vec2i candidate{rng.nextInt(0, gridWidth_ - 1), rng.nextInt(0, gridHeight_ - 1)};
        if (isCellFree(candidate, occupiedCells)) {
            Item item;
            item.position = candidate;
            item.effect = catalog_.rollRandomEffect(rng, luck);
            items_.push_back(item);
            return;
        }
    }
}

void ItemSpawner::removeItemAt(const Vec2i& pos) {
    items_.erase(std::remove_if(items_.begin(), items_.end(),
                                 [&](const Item& item) { return item.position == pos; }),
                 items_.end());
}

}  // namespace snake_engine

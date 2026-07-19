#include "snake_engine/upgrades.hpp"

#include <cmath>

#include "snake_engine/math.hpp"

namespace snake_engine {

const char* toString(UpgradeKind kind) {
    switch (kind) {
        case UpgradeKind::Vitality:
            return "Vitality";
        case UpgradeKind::Luck:
            return "Luck";
        case UpgradeKind::Insurance:
            return "Insurance";
        case UpgradeKind::Regeneration:
            return "Regeneration";
        default:
            return "Unknown";
    }
}

int UpgradeLevels::level(UpgradeKind kind) const {
    switch (kind) {
        case UpgradeKind::Vitality:
            return vitality;
        case UpgradeKind::Luck:
            return luck;
        case UpgradeKind::Insurance:
            return insurance;
        case UpgradeKind::Regeneration:
            return regeneration;
        default:
            return 0;
    }
}

void UpgradeLevels::setLevel(UpgradeKind kind, int level) {
    switch (kind) {
        case UpgradeKind::Vitality:
            vitality = level;
            break;
        case UpgradeKind::Luck:
            luck = level;
            break;
        case UpgradeKind::Insurance:
            insurance = level;
            break;
        case UpgradeKind::Regeneration:
            regeneration = level;
            break;
        default:
            break;
    }
}

PlayerStatModifiers computeModifiers(const UpgradeLevels& levels) {
    PlayerStatModifiers mods;
    mods.maxLifeBonus = static_cast<float>(levels.vitality) * 10.0f;
    mods.luck = clamp(static_cast<float>(levels.luck) * 0.12f, 0.0f, 1.0f);
    mods.startingShields = levels.insurance;
    mods.regenPerSecond = static_cast<float>(levels.regeneration) * 0.15f;
    return mods;
}

int64_t costForNextLevel(UpgradeKind /*kind*/, int currentLevel) {
    double base = 50.0;
    double cost = base * std::pow(static_cast<double>(currentLevel) + 1.0, 1.5);
    return static_cast<int64_t>(std::round(cost));
}

bool MetaProgress::purchase(UpgradeKind kind) {
    int current = upgrades.level(kind);
    int64_t cost = costForNextLevel(kind, current);
    if (essence < cost) {
        return false;
    }
    essence -= cost;
    upgrades.setLevel(kind, current + 1);
    return true;
}

int64_t MetaProgress::essenceForScore(int64_t score) {
    return score / 4;
}

}  // namespace snake_engine

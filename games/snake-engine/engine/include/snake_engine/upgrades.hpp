#pragma once

#include <cstdint>
#include <string>

namespace snake_engine {

// Persistent, between-run meta-progression. Spending "Essence" (earned from
// a run's final score) on these upgrades is the "bonus upgrades" system —
// distinct from the in-run, algorithmically-rolled item effects.
enum class UpgradeKind {
    Vitality,      // +10 max life per level
    Luck,          // nudges future item rolls toward milder drawbacks / stronger bonuses
    Insurance,     // +1 starting shield charge (absorbs one drawback fully)
    Regeneration,  // passive life regen per second
    Count,
};

const char* toString(UpgradeKind kind);

struct UpgradeLevels {
    int vitality = 0;
    int luck = 0;
    int insurance = 0;
    int regeneration = 0;

    [[nodiscard]] int level(UpgradeKind kind) const;
    void setLevel(UpgradeKind kind, int level);
};

// Derived, ready-to-use gameplay numbers for the current upgrade levels.
struct PlayerStatModifiers {
    float maxLifeBonus = 0.0f;
    float luck = 0.0f;  // clamped [0, 1]
    int startingShields = 0;
    float regenPerSecond = 0.0f;
};

PlayerStatModifiers computeModifiers(const UpgradeLevels& levels);

// Essence cost to go from `currentLevel` to `currentLevel + 1`.
int64_t costForNextLevel(UpgradeKind kind, int currentLevel);

class MetaProgress {
public:
    int64_t essence = 0;
    UpgradeLevels upgrades;
    std::string themeId = "classic";

    // Returns true and deducts essence if affordable, false (no-op) if not.
    bool purchase(UpgradeKind kind);

    // Essence awarded for a finished run's score.
    static int64_t essenceForScore(int64_t score);
};

}  // namespace snake_engine

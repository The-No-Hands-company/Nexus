#pragma once

#include <string>
#include <vector>

#include "snake_engine/effect.hpp"
#include "snake_engine/rng.hpp"

namespace snake_engine {

// One category's full definition: how likely it is to be picked at all, how
// likely each polarity is, and the severity ladder for each polarity. This
// is exactly the shape SnakeED's future effect editor will read and write —
// keeping it plain data (no behavior) means the editor never needs engine
// recompiles to add or retune an effect.
struct EffectDef {
    std::string id;
    std::string displayName;
    EffectCategory category = EffectCategory::Life;
    double spawnWeight = 1.0;
    double bonusWeight = 1.0;
    double drawbackWeight = 1.0;
    std::vector<SeverityTier> bonusTiers;
    std::vector<SeverityTier> drawbackTiers;
};

class EffectCatalog {
public:
    // Loads definitions from a JSON file on disk. Returns false (and leaves
    // the catalog with its built-in fallback defaults) if the file can't be
    // read or parsed, so a broken data file never crashes the game.
    bool loadFromFile(const std::string& path);

    // Populates a small built-in set of effects so the game is playable even
    // with no data file present (e.g. a fresh `cmake --build .`).
    void loadBuiltinDefaults();

    [[nodiscard]] const std::vector<EffectDef>& definitions() const { return defs_; }

    // Draws a random effect: which category, which polarity, and which
    // severity tier — never the player's choice. `luck` in [0,1] nudges the
    // tier roll (see effect_catalog.cpp for the exact curve) and comes from
    // the player's persistent upgrades.
    RolledEffect rollRandomEffect(Rng& rng, float luck) const;

private:
    std::vector<EffectDef> defs_;
};

}  // namespace snake_engine

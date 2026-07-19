#pragma once

#include <string>

namespace snake_engine {

// A category groups related bonus/drawback pairs. The editor (SnakeED) will
// eventually let designers add new categories without touching engine code —
// for now this enum covers the launch set, and effect_catalog.hpp reads the
// rest of the tuning data from JSON.
enum class EffectCategory {
    Life,
    Speed,
    Size,
    Score,
    Control,
    Vision,
    Shield,
};

enum class EffectPolarity {
    Bonus,
    Drawback,
};

// How a tier's magnitude is interpreted when applied.
enum class MagnitudeType {
    FlatLife,             // absolute life delta, e.g. -1
    PercentOfCurrentLife,  // fraction of the player's life AT PICKUP TIME, e.g. 0.99 -> -99%
    FlatScore,
    FlatValue,             // generic scalar for non-life categories (speed mult, radius, etc.)
};

// One severity rung within a category+polarity, e.g. "Catastrophic" drawback
// on Life = 99% of current life, weight 1 (rare). Tiers are authored in JSON
// (data/effects/*.json) and rolled by EffectCatalog — the player never picks
// the tier, it is drawn algorithmically at spawn time.
struct SeverityTier {
    std::string name;
    double weight = 1.0;
    MagnitudeType magnitudeType = MagnitudeType::FlatValue;
    double magnitude = 0.0;
    float durationSeconds = 0.0f;  // 0 = instant, >0 = timed buff/debuff
};

// The concrete, already-rolled effect an Item carries. `magnitude` mirrors
// the tier's authored value; life-affecting categories resolve the *actual*
// life delta at consumption time via Game::applyRolledEffect, since percent
// tiers are defined relative to the player's life at the moment of pickup.
struct RolledEffect {
    std::string defId;
    std::string tierName;
    EffectCategory category = EffectCategory::Life;
    EffectPolarity polarity = EffectPolarity::Bonus;
    MagnitudeType magnitudeType = MagnitudeType::FlatValue;
    double magnitude = 0.0;
    float durationSeconds = 0.0f;
};

const char* toString(EffectCategory category);
const char* toString(EffectPolarity polarity);

// Pure function so the "-1 flat life ... up to -99% of current life at
// pickup time" rule can be unit-tested without spinning up a Game/SDL. Not
// clamped to [0, maxLife] — the caller (Game::applyRolledEffect) does that.
float resolveLifeDelta(MagnitudeType type, double magnitude, float currentLife);

}  // namespace snake_engine

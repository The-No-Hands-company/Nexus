#include "snake_engine/effect_catalog.hpp"

#include <SDL.h>

#include <algorithm>
#include <nlohmann/json.hpp>

#include "snake_engine/math.hpp"

namespace snake_engine {

namespace {

using json = nlohmann::json;

EffectCategory categoryFromString(const std::string& s) {
    if (s == "Life") return EffectCategory::Life;
    if (s == "Speed") return EffectCategory::Speed;
    if (s == "Size") return EffectCategory::Size;
    if (s == "Score") return EffectCategory::Score;
    if (s == "Control") return EffectCategory::Control;
    if (s == "Vision") return EffectCategory::Vision;
    if (s == "Shield") return EffectCategory::Shield;
    return EffectCategory::Life;
}

MagnitudeType magnitudeTypeFromString(const std::string& s) {
    if (s == "flat_life") return MagnitudeType::FlatLife;
    if (s == "percent_of_current_life") return MagnitudeType::PercentOfCurrentLife;
    if (s == "flat_score") return MagnitudeType::FlatScore;
    return MagnitudeType::FlatValue;
}

std::vector<SeverityTier> parseTiers(const json& arr) {
    std::vector<SeverityTier> tiers;
    for (const auto& t : arr) {
        SeverityTier tier;
        tier.name = t.value("name", "Unnamed");
        tier.weight = t.value("weight", 1.0);
        tier.magnitudeType = magnitudeTypeFromString(t.value("magnitude_type", "flat_value"));
        tier.magnitude = t.value("magnitude", 0.0);
        tier.durationSeconds = t.value("duration_seconds", 0.0f);
        tiers.push_back(tier);
    }
    return tiers;
}

}  // namespace

namespace {

// Reads the whole file through SDL_RWops rather than std::ifstream: on
// Android that transparently falls back to the APK's packaged assets/ (a
// plain relative path can't reach those via libc fopen at all), and on
// Emscripten it reads the virtual filesystem set up by --preload-file. On
// desktop it behaves like a normal file read.
bool readWholeFile(const std::string& path, std::string& outContents) {
    SDL_RWops* rw = SDL_RWFromFile(path.c_str(), "rb");
    if (rw == nullptr) {
        return false;
    }
    Sint64 size = SDL_RWsize(rw);
    if (size < 0) {
        SDL_RWclose(rw);
        return false;
    }
    outContents.resize(static_cast<size_t>(size));
    size_t readBytes = SDL_RWread(rw, outContents.data(), 1, static_cast<size_t>(size));
    SDL_RWclose(rw);
    return readBytes == static_cast<size_t>(size);
}

}  // namespace

bool EffectCatalog::loadFromFile(const std::string& path) {
    std::string contents;
    if (!readWholeFile(path, contents)) {
        return false;
    }

    json root;
    try {
        root = json::parse(contents);
    } catch (const json::parse_error&) {
        return false;
    }

    if (!root.contains("effects") || !root["effects"].is_array()) {
        return false;
    }

    std::vector<EffectDef> parsed;
    for (const auto& e : root["effects"]) {
        EffectDef def;
        def.id = e.value("id", "unknown");
        def.displayName = e.value("display_name", def.id);
        def.category = categoryFromString(e.value("category", "Life"));
        def.spawnWeight = e.value("spawn_weight", 1.0);

        if (e.contains("polarity_weights")) {
            def.bonusWeight = e["polarity_weights"].value("bonus", 1.0);
            def.drawbackWeight = e["polarity_weights"].value("drawback", 1.0);
        }
        if (e.contains("bonus_tiers")) {
            def.bonusTiers = parseTiers(e["bonus_tiers"]);
        }
        if (e.contains("drawback_tiers")) {
            def.drawbackTiers = parseTiers(e["drawback_tiers"]);
        }

        if (def.bonusTiers.empty() && def.drawbackTiers.empty()) {
            continue;
        }
        parsed.push_back(std::move(def));
    }

    if (parsed.empty()) {
        return false;
    }

    defs_ = std::move(parsed);
    return true;
}

void EffectCatalog::loadBuiltinDefaults() {
    defs_.clear();

    EffectDef life;
    life.id = "life";
    life.displayName = "Life Force";
    life.category = EffectCategory::Life;
    life.spawnWeight = 3.0;
    life.bonusWeight = 55.0;
    life.drawbackWeight = 45.0;
    life.bonusTiers = {
        {"Snack", 45.0, MagnitudeType::FlatLife, 1.0, 0.0f},
        {"Meal", 30.0, MagnitudeType::FlatLife, 3.0, 0.0f},
        {"Feast", 18.0, MagnitudeType::PercentOfCurrentLife, 0.25, 0.0f},
        {"Vitality Surge", 6.0, MagnitudeType::PercentOfCurrentLife, 0.50, 0.0f},
        {"Rebirth", 1.0, MagnitudeType::PercentOfCurrentLife, 1.00, 0.0f},
    };
    life.drawbackTiers = {
        {"Trivial", 40.0, MagnitudeType::FlatLife, -1.0, 0.0f},
        {"Minor", 25.0, MagnitudeType::PercentOfCurrentLife, -0.05, 0.0f},
        {"Moderate", 16.0, MagnitudeType::PercentOfCurrentLife, -0.15, 0.0f},
        {"Major", 10.0, MagnitudeType::PercentOfCurrentLife, -0.35, 0.0f},
        {"Severe", 6.0, MagnitudeType::PercentOfCurrentLife, -0.60, 0.0f},
        {"Critical", 2.5, MagnitudeType::PercentOfCurrentLife, -0.85, 0.0f},
        {"Catastrophic", 0.5, MagnitudeType::PercentOfCurrentLife, -0.99, 0.0f},
    };
    defs_.push_back(life);

    EffectDef speed;
    speed.id = "speed";
    speed.displayName = "Momentum";
    speed.category = EffectCategory::Speed;
    speed.spawnWeight = 2.0;
    speed.bonusWeight = 50.0;
    speed.drawbackWeight = 50.0;
    speed.bonusTiers = {
        {"Quickstep", 60.0, MagnitudeType::FlatValue, 1.15, 6.0f},
        {"Sprint", 30.0, MagnitudeType::FlatValue, 1.35, 6.0f},
        {"Blitz", 10.0, MagnitudeType::FlatValue, 1.60, 5.0f},
    };
    speed.drawbackTiers = {
        {"Sluggish", 55.0, MagnitudeType::FlatValue, 0.80, 6.0f},
        {"Molasses", 30.0, MagnitudeType::FlatValue, 0.60, 6.0f},
        {"Frenzy", 15.0, MagnitudeType::FlatValue, 1.90, 5.0f},
    };
    defs_.push_back(speed);

    EffectDef size;
    size.id = "size";
    size.displayName = "Mass";
    size.category = EffectCategory::Size;
    size.spawnWeight = 2.0;
    size.bonusWeight = 60.0;
    size.drawbackWeight = 40.0;
    size.bonusTiers = {
        {"Growth", 70.0, MagnitudeType::FlatValue, 2.0, 0.0f},
        {"Overgrowth", 25.0, MagnitudeType::FlatValue, 5.0, 0.0f},
        {"Colossus", 5.0, MagnitudeType::FlatValue, 9.0, 0.0f},
    };
    size.drawbackTiers = {
        {"Shrink", 60.0, MagnitudeType::FlatValue, -2.0, 0.0f},
        {"Wither", 30.0, MagnitudeType::FlatValue, -4.0, 0.0f},
        {"Atrophy", 10.0, MagnitudeType::FlatValue, -7.0, 0.0f},
    };
    defs_.push_back(size);

    EffectDef score;
    score.id = "score";
    score.displayName = "Fortune";
    score.category = EffectCategory::Score;
    score.spawnWeight = 2.0;
    score.bonusWeight = 60.0;
    score.drawbackWeight = 40.0;
    score.bonusTiers = {
        {"Bounty", 55.0, MagnitudeType::FlatScore, 25.0, 0.0f},
        {"Jackpot", 35.0, MagnitudeType::FlatScore, 75.0, 0.0f},
        {"Windfall", 10.0, MagnitudeType::FlatScore, 200.0, 0.0f},
    };
    score.drawbackTiers = {
        {"Toll", 60.0, MagnitudeType::FlatScore, -20.0, 0.0f},
        {"Fine", 30.0, MagnitudeType::FlatScore, -60.0, 0.0f},
        {"Forfeiture", 10.0, MagnitudeType::FlatScore, -150.0, 0.0f},
    };
    defs_.push_back(score);

    EffectDef control;
    control.id = "control";
    control.displayName = "Steadiness";
    control.category = EffectCategory::Control;
    control.spawnWeight = 1.5;
    control.bonusWeight = 50.0;
    control.drawbackWeight = 50.0;
    control.bonusTiers = {
        {"Precision", 70.0, MagnitudeType::FlatValue, 0.5, 8.0f},
        {"Steady Hand", 30.0, MagnitudeType::FlatValue, 0.25, 8.0f},
    };
    control.drawbackTiers = {
        {"Wobble", 60.0, MagnitudeType::FlatValue, 0.35, 6.0f},
        {"Static", 30.0, MagnitudeType::FlatValue, 0.65, 6.0f},
        {"Chaos", 10.0, MagnitudeType::FlatValue, 1.0, 4.0f},
    };
    defs_.push_back(control);

    EffectDef vision;
    vision.id = "vision";
    vision.displayName = "Clarity";
    vision.category = EffectCategory::Vision;
    vision.spawnWeight = 1.5;
    vision.bonusWeight = 50.0;
    vision.drawbackWeight = 50.0;
    vision.bonusTiers = {
        {"Clear Sight", 70.0, MagnitudeType::FlatValue, 1.3, 10.0f},
        {"Farsight", 30.0, MagnitudeType::FlatValue, 1.6, 8.0f},
    };
    vision.drawbackTiers = {
        {"Haze", 60.0, MagnitudeType::FlatValue, 0.75, 8.0f},
        {"Fog", 30.0, MagnitudeType::FlatValue, 0.55, 6.0f},
        {"Blackout", 10.0, MagnitudeType::FlatValue, 0.35, 4.0f},
    };
    defs_.push_back(vision);

    EffectDef shield;
    shield.id = "shield";
    shield.displayName = "Ward";
    shield.category = EffectCategory::Shield;
    shield.spawnWeight = 1.0;
    shield.bonusWeight = 100.0;
    shield.drawbackWeight = 0.0;
    shield.bonusTiers = {
        {"Ward", 75.0, MagnitudeType::FlatValue, 1.0, 0.0f},
        {"Aegis", 25.0, MagnitudeType::FlatValue, 2.0, 0.0f},
    };
    defs_.push_back(shield);
}

RolledEffect EffectCatalog::rollRandomEffect(Rng& rng, float luck) const {
    luck = clamp(luck, 0.0f, 1.0f);

    if (defs_.empty()) {
        return RolledEffect{};
    }

    std::vector<double> defWeights;
    defWeights.reserve(defs_.size());
    for (const auto& d : defs_) defWeights.push_back(d.spawnWeight);
    const EffectDef& def = defs_[rng.weightedIndex(defWeights)];

    // Luck nudges the odds of getting a bonus instead of a drawback.
    double bonusWeight = def.bonusWeight * (1.0 + 0.5 * luck);
    double drawbackWeight = def.drawbackWeight * (1.0 - 0.35 * luck);
    bool haveBonusTiers = !def.bonusTiers.empty();
    bool haveDrawbackTiers = !def.drawbackTiers.empty();
    if (!haveBonusTiers) bonusWeight = 0.0;
    if (!haveDrawbackTiers) drawbackWeight = 0.0;

    bool isBonus = true;
    if (bonusWeight <= 0.0 && drawbackWeight <= 0.0) {
        isBonus = true;
    } else {
        double roll = static_cast<double>(rng.nextFloat01()) * (bonusWeight + drawbackWeight);
        isBonus = roll <= bonusWeight;
    }

    const std::vector<SeverityTier>& tiers = isBonus ? def.bonusTiers : def.drawbackTiers;
    if (tiers.empty()) {
        return RolledEffect{def.id, "None", def.category, EffectPolarity::Bonus,
                             MagnitudeType::FlatValue, 0.0, 0.0f};
    }

    // Luck also reshapes the severity ladder itself: for drawbacks it damps
    // down the weight of the nastier (higher-index) tiers; for bonuses it
    // boosts the weight of the stronger (higher-index) tiers. Tier order in
    // the data file is always mild -> severe / weak -> strong.
    std::vector<double> tierWeights;
    tierWeights.reserve(tiers.size());
    size_t n = tiers.size();
    for (size_t i = 0; i < n; ++i) {
        double severityFactor = n > 1 ? static_cast<double>(i) / static_cast<double>(n - 1) : 0.0;
        double w = tiers[i].weight;
        if (!isBonus) {
            w *= (1.0 - 0.7 * luck * severityFactor);
        } else {
            w *= (1.0 + 0.7 * luck * severityFactor);
        }
        tierWeights.push_back(std::max(w, 0.0001));
    }
    const SeverityTier& tier = tiers[rng.weightedIndex(tierWeights)];

    RolledEffect result;
    result.defId = def.id;
    result.tierName = tier.name;
    result.category = def.category;
    result.polarity = isBonus ? EffectPolarity::Bonus : EffectPolarity::Drawback;
    result.magnitudeType = tier.magnitudeType;
    result.magnitude = tier.magnitude;
    result.durationSeconds = tier.durationSeconds;
    return result;
}

}  // namespace snake_engine

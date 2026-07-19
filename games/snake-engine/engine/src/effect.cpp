#include "snake_engine/effect.hpp"

namespace snake_engine {

const char* toString(EffectCategory category) {
    switch (category) {
        case EffectCategory::Life:
            return "Life";
        case EffectCategory::Speed:
            return "Speed";
        case EffectCategory::Size:
            return "Size";
        case EffectCategory::Score:
            return "Score";
        case EffectCategory::Control:
            return "Control";
        case EffectCategory::Vision:
            return "Vision";
        case EffectCategory::Shield:
            return "Shield";
    }
    return "Unknown";
}

const char* toString(EffectPolarity polarity) {
    return polarity == EffectPolarity::Bonus ? "Bonus" : "Drawback";
}

float resolveLifeDelta(MagnitudeType type, double magnitude, float currentLife) {
    if (type == MagnitudeType::FlatLife) {
        return static_cast<float>(magnitude);
    }
    if (type == MagnitudeType::PercentOfCurrentLife) {
        return currentLife * static_cast<float>(magnitude);
    }
    return 0.0f;
}

}  // namespace snake_engine

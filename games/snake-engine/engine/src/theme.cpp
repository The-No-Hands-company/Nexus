#include "snake_engine/theme.hpp"

namespace snake_engine {

SDL_Color Theme::colorForEffect(EffectCategory category, EffectPolarity polarity) const {
    size_t index = static_cast<size_t>(category);
    if (index >= bonusColors.size()) {
        return hudText;
    }
    if (category == EffectCategory::Shield) {
        return shieldColor;
    }
    return polarity == EffectPolarity::Bonus ? bonusColors[index] : drawbackColors[index];
}

}  // namespace snake_engine

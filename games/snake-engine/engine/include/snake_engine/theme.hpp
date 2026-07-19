#pragma once

#include <SDL.h>

#include <array>
#include <string>

#include "snake_engine/effect.hpp"

namespace snake_engine {

// How a filled cell (snake segment, item, HUD swatch) is drawn. Flat is the
// original look — solid color, thin grid lines, nothing fancy. Bevel3D adds
// a gradient fill, a raised-edge bevel, a soft drop shadow, and a specular
// highlight to fake a glossy 3D block out of plain 2D rect fills.
enum class BlockStyle {
    Flat,
    Bevel3D,
};

enum class BackgroundStyle {
    Solid,
    VerticalGradient,
};

// One theme's full palette + rendering style. Plain data, loaded from
// data/themes/themes.json by ThemeCatalog — the same "data, not code" shape
// as EffectCatalog, so growing the theme library never needs a recompile
// and is exactly what a future SnakeED theme editor would read and write.
struct Theme {
    std::string id;
    std::string displayName;
    BlockStyle blockStyle = BlockStyle::Flat;
    BackgroundStyle backgroundStyle = BackgroundStyle::Solid;

    SDL_Color backgroundTop{18, 18, 24, 255};
    SDL_Color backgroundBottom{18, 18, 24, 255};  // == backgroundTop when Solid
    SDL_Color gridLine{32, 32, 40, 255};
    SDL_Color snakeHead{120, 230, 140, 255};
    SDL_Color snakeBody{70, 170, 100, 255};
    SDL_Color hudBarBackground{40, 40, 48, 255};
    SDL_Color hudText{235, 235, 240, 255};
    SDL_Color vignette{0, 0, 0, 255};
    SDL_Color shieldColor{235, 235, 240, 255};

    // 0 = no glow (Flat themes typically leave this at 0); ~0.6-1.2 for a
    // visible but tasteful glow on items/snake head; higher gets gaudy.
    float glowIntensity = 0.0f;

    // Indexed by EffectCategory; Shield only ever spawns as a bonus but the
    // drawback slot is kept for a uniform lookup.
    std::array<SDL_Color, 7> bonusColors{};
    std::array<SDL_Color, 7> drawbackColors{};

    [[nodiscard]] SDL_Color colorForEffect(EffectCategory category, EffectPolarity polarity) const;
};

}  // namespace snake_engine

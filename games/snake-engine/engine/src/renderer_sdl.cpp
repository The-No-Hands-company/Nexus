#include "snake_engine/renderer.hpp"

#include <algorithm>
#include <array>
#include <cmath>

namespace snake_engine {

namespace {

SDL_Color mixColor(SDL_Color a, SDL_Color b, float t) {
    t = clamp(t, 0.0f, 1.0f);
    auto lerp = [&](Uint8 x, Uint8 y) {
        return static_cast<Uint8>(static_cast<float>(x) + (static_cast<float>(y) - x) * t);
    };
    return SDL_Color{lerp(a.r, b.r), lerp(a.g, b.g), lerp(a.b, b.b), lerp(a.a, b.a)};
}

SDL_Color lighten(SDL_Color c, float amount) {
    return mixColor(c, SDL_Color{255, 255, 255, c.a}, amount);
}

SDL_Color darken(SDL_Color c, float amount) {
    return mixColor(c, SDL_Color{0, 0, 0, c.a}, amount);
}

SDL_Color withAlpha(SDL_Color c, Uint8 a) {
    c.a = a;
    return c;
}

SDL_Color lifeBarColor(float ratio) {
    ratio = clamp(ratio, 0.0f, 1.0f);
    if (ratio > 0.5f) {
        return SDL_Color{static_cast<Uint8>(255 * (1.0f - (ratio - 0.5f) * 2.0f)), 210, 90, 255};
    }
    return SDL_Color{230, static_cast<Uint8>(210 * (ratio * 2.0f)), 60, 255};
}

// Seven-segment layout: bit order = {top, topRight, bottomRight, bottom,
// bottomLeft, topLeft, middle}.
constexpr std::array<std::array<bool, 7>, 10> kDigitSegments{{
    {true, true, true, true, true, true, false},     // 0
    {false, true, true, false, false, false, false},  // 1
    {true, true, false, true, true, false, true},     // 2
    {true, true, true, true, false, false, true},      // 3
    {false, true, true, false, false, true, true},     // 4
    {true, false, true, true, false, true, true},      // 5
    {true, false, true, true, true, true, true},       // 6
    {true, true, true, false, false, false, false},    // 7
    {true, true, true, true, true, true, true},        // 8
    {true, true, true, true, false, true, true},       // 9
}};

}  // namespace

GameRenderer::GameRenderer(SDL_Renderer* renderer) : renderer_(renderer), theme_(&defaultTheme_) {
    // Safe for every existing opaque (alpha=255) draw call too: blended with
    // alpha=255 is bit-for-bit the same as an overwrite, so enabling this
    // globally doesn't change how the Flat/Classic style looks.
    SDL_SetRenderDrawBlendMode(renderer_, SDL_BLENDMODE_BLEND);
}

void GameRenderer::fillRect(int x, int y, int w, int h, SDL_Color color) {
    SDL_SetRenderDrawColor(renderer_, color.r, color.g, color.b, color.a);
    SDL_Rect rect{x, y, w, h};
    SDL_RenderFillRect(renderer_, &rect);
}

void GameRenderer::fillGradientRectV(int x, int y, int w, int h, SDL_Color top, SDL_Color bottom) {
    if (h <= 0 || w <= 0) return;
    for (int row = 0; row < h; ++row) {
        float t = h > 1 ? static_cast<float>(row) / static_cast<float>(h - 1) : 0.0f;
        SDL_Color c = mixColor(top, bottom, t);
        SDL_SetRenderDrawColor(renderer_, c.r, c.g, c.b, c.a);
        SDL_RenderDrawLine(renderer_, x, y + row, x + w - 1, y + row);
    }
}

void GameRenderer::drawDigits(int x, int y, long long value, int digitWidth, int digitHeight,
                               int thickness, SDL_Color color) {
    bool negative = value < 0;
    if (negative) value = -value;

    std::string digits = std::to_string(value);
    int cursor = x + (negative ? digitWidth : 0);
    if (negative) {
        fillRect(x, y + digitHeight / 2 - thickness / 2, digitWidth, thickness, color);
    }

    for (char c : digits) {
        int d = c - '0';
        const auto& segs = kDigitSegments[static_cast<size_t>(d)];
        int w = digitWidth;
        int h = digitHeight;
        int t = thickness;
        // top
        if (segs[0]) fillRect(cursor, y, w, t, color);
        // top-right
        if (segs[1]) fillRect(cursor + w - t, y, t, h / 2, color);
        // bottom-right
        if (segs[2]) fillRect(cursor + w - t, y + h / 2, t, h / 2, color);
        // bottom
        if (segs[3]) fillRect(cursor, y + h - t, w, t, color);
        // bottom-left
        if (segs[4]) fillRect(cursor, y + h / 2, t, h / 2, color);
        // top-left
        if (segs[5]) fillRect(cursor, y, t, h / 2, color);
        // middle
        if (segs[6]) fillRect(cursor, y + h / 2 - t / 2, w, t, color);

        cursor += digitWidth + thickness * 2;
    }
}

// --- block drawing ----------------------------------------------------------

void GameRenderer::drawBlockFlat(int x, int y, int size, SDL_Color color) {
    fillRect(x, y, size, size, color);
}

void GameRenderer::drawBlockBevel3D(int x, int y, int size, SDL_Color color, const Theme& style,
                                     bool glow) {
    if (glow && style.glowIntensity > 0.0f) {
        constexpr int kGlowSteps = 4;
        for (int i = kGlowSteps; i >= 1; --i) {
            float t = static_cast<float>(i) / static_cast<float>(kGlowSteps);
            Uint8 alpha = static_cast<Uint8>(clamp(style.glowIntensity * 55.0f * (1.0f - t) + 10.0f,
                                                     0.0f, 90.0f));
            int pad = i * std::max(1, size / 8);
            fillRect(x - pad, y - pad, size + pad * 2, size + pad * 2, withAlpha(color, alpha));
        }
    }

    // Drop shadow, offset down-right so the block reads as "raised".
    int shadowOffset = std::max(1, size / 10);
    fillRect(x + shadowOffset, y + shadowOffset, size, size, SDL_Color{0, 0, 0, 90});

    fillGradientRectV(x, y, size, size, lighten(color, 0.22f), darken(color, 0.38f));

    int bevel = std::max(1, size / 9);
    SDL_Color hi = lighten(color, 0.65f);
    SDL_Color lo = darken(color, 0.55f);
    fillRect(x, y, size, bevel, hi);              // top
    fillRect(x, y, bevel, size, hi);               // left
    fillRect(x, y + size - bevel, size, bevel, lo); // bottom
    fillRect(x + size - bevel, y, bevel, size, lo); // right

    // Small glossy highlight in the upper-left, like light catching a
    // rounded gem — this one small touch does most of the work of reading
    // as "3D" rather than "rectangle with a border".
    int specW = std::max(2, size / 2);
    int specH = std::max(2, size / 3);
    fillRect(x + bevel + 1, y + bevel + 1, specW, specH, withAlpha(SDL_Color{255, 255, 255, 255}, 60));
}

void GameRenderer::drawBlock(int x, int y, int size, SDL_Color color, const Theme& style,
                              bool glow) {
    if (style.blockStyle == BlockStyle::Bevel3D) {
        drawBlockBevel3D(x, y, size, color, style, glow);
    } else {
        drawBlockFlat(x, y, size, color);
    }
}

// --- background / vignette --------------------------------------------------

void GameRenderer::drawBackground(int width, int height) {
    if (theme_->backgroundStyle == BackgroundStyle::VerticalGradient) {
        fillGradientRectV(0, 0, width, height, theme_->backgroundTop, theme_->backgroundBottom);
    } else {
        SDL_SetRenderDrawColor(renderer_, theme_->backgroundTop.r, theme_->backgroundTop.g,
                                theme_->backgroundTop.b, theme_->backgroundTop.a);
        SDL_RenderClear(renderer_);
    }
}

void GameRenderer::drawVignette(int width, int height) {
    if (theme_->blockStyle != BlockStyle::Bevel3D) {
        return;
    }
    // Per-pixel falloff (one alpha-blended 1px line per step) rather than a
    // handful of nested rectangles — with only a dozen or so discrete steps
    // the rectangle approach shows visible banding, especially on light
    // theme backgrounds (Pastel Dream) where the eye is much more sensitive
    // to small alpha jumps than it is on a near-black background.
    int thickness = std::min(width, height) / 5;
    for (int i = 0; i < thickness; ++i) {
        float t = 1.0f - static_cast<float>(i) / static_cast<float>(thickness);
        Uint8 alpha = static_cast<Uint8>(clamp(t * t * 100.0f, 0.0f, 100.0f));
        SDL_Color c = withAlpha(theme_->vignette, alpha);
        SDL_SetRenderDrawColor(renderer_, c.r, c.g, c.b, c.a);
        SDL_RenderDrawLine(renderer_, 0, i, width - 1, i);
        SDL_RenderDrawLine(renderer_, 0, height - 1 - i, width - 1, height - 1 - i);
        SDL_RenderDrawLine(renderer_, i, 0, i, height - 1);
        SDL_RenderDrawLine(renderer_, width - 1 - i, 0, width - 1 - i, height - 1);
    }
}

// --- game rendering ----------------------------------------------------------

void GameRenderer::drawBoard(const Game& game) {
    const Grid& grid = game.grid();
    int width = 0, height = 0;
    SDL_GetRendererOutputSize(renderer_, &width, &height);

    drawBackground(width, height);

    SDL_SetRenderDrawColor(renderer_, theme_->gridLine.r, theme_->gridLine.g, theme_->gridLine.b,
                            theme_->gridLine.a);
    for (int x = 0; x <= grid.width; ++x) {
        SDL_RenderDrawLine(renderer_, x * grid.cellSizePixels, 0, x * grid.cellSizePixels,
                            grid.heightPixels());
    }
    for (int y = 0; y <= grid.height; ++y) {
        SDL_RenderDrawLine(renderer_, 0, y * grid.cellSizePixels, grid.widthPixels(),
                            y * grid.cellSizePixels);
    }

    drawVignette(width, height);
}

void GameRenderer::drawSnake(const Game& game) {
    const Grid& grid = game.grid();
    const auto& body = game.snake().body();
    int inset = std::max(1, grid.cellSizePixels / 10);
    bool first = true;
    for (const auto& segment : body) {
        SDL_Color color = first ? theme_->snakeHead : theme_->snakeBody;
        drawBlock(segment.x * grid.cellSizePixels + inset, segment.y * grid.cellSizePixels + inset,
                  grid.cellSizePixels - inset * 2, color, *theme_, first);
        first = false;
    }
}

void GameRenderer::drawItems(const Game& game) {
    const Grid& grid = game.grid();
    int inset = std::max(2, grid.cellSizePixels / 6);
    for (const auto& item : game.itemSpawner().items()) {
        SDL_Color color = theme_->colorForEffect(item.effect.category, item.effect.polarity);
        int x = item.position.x * grid.cellSizePixels + inset;
        int y = item.position.y * grid.cellSizePixels + inset;
        int size = grid.cellSizePixels - inset * 2;
        drawBlock(x, y, size, color, *theme_, true);
        if (item.effect.polarity == EffectPolarity::Drawback) {
            SDL_SetRenderDrawColor(renderer_, 0, 0, 0, 255);
            SDL_Rect outline{x, y, size, size};
            SDL_RenderDrawRect(renderer_, &outline);
        }
    }
}

void GameRenderer::drawVisionOverlay(const Game& game) {
    float vision = clamp(game.visionMultiplier(), 0.2f, 1.0f);
    if (vision >= 0.999f) {
        return;
    }
    const Grid& grid = game.grid();
    int maxThickness = std::min(grid.widthPixels(), grid.heightPixels()) / 3;
    int thickness = static_cast<int>(static_cast<float>(maxThickness) * (1.0f - vision));
    if (thickness <= 0) return;

    SDL_SetRenderDrawColor(renderer_, 0, 0, 0, 190);
    SDL_Rect top{0, 0, grid.widthPixels(), thickness};
    SDL_Rect bottom{0, grid.heightPixels() - thickness, grid.widthPixels(), thickness};
    SDL_Rect left{0, 0, thickness, grid.heightPixels()};
    SDL_Rect right{grid.widthPixels() - thickness, 0, thickness, grid.heightPixels()};
    SDL_RenderFillRect(renderer_, &top);
    SDL_RenderFillRect(renderer_, &bottom);
    SDL_RenderFillRect(renderer_, &left);
    SDL_RenderFillRect(renderer_, &right);
}

void GameRenderer::drawHud(const Game& game) {
    const Grid& grid = game.grid();
    int barX = 12;
    int barY = grid.heightPixels() + 10;
    int barW = 220;
    int barH = 18;

    fillRect(barX, barY, barW, barH, theme_->hudBarBackground);
    float ratio = game.maxLife() > 0.0f ? game.life() / game.maxLife() : 0.0f;
    int fillW = static_cast<int>(static_cast<float>(barW) * clamp(ratio, 0.0f, 1.0f));
    SDL_Color barColor = lifeBarColor(ratio);
    if (theme_->blockStyle == BlockStyle::Bevel3D) {
        fillGradientRectV(barX, barY, fillW, barH, lighten(barColor, 0.3f), darken(barColor, 0.25f));
    } else {
        fillRect(barX, barY, fillW, barH, barColor);
    }

    drawDigits(barX + barW + 16, barY - 3, static_cast<long long>(std::lround(game.life())), 10,
               24, 4, theme_->hudText);

    drawDigits(barX, barY + barH + 14, static_cast<long long>(game.score()), 12, 28, 4,
               theme_->hudText);

    for (int i = 0; i < game.shieldCharges(); ++i) {
        drawBlock(barX + 260 + i * 20, barY, 14, theme_->shieldColor, *theme_, false);
    }
}

void GameRenderer::drawEffectBadges(const Game& game) {
    const Grid& grid = game.grid();
    int x = 12;
    int y = grid.heightPixels() + 60;
    for (const auto& active : game.activeEffects()) {
        SDL_Color color = theme_->colorForEffect(active.effect.category, active.effect.polarity);
        drawBlock(x, y, 14, color, *theme_, false);
        x += 22;
    }
}

void GameRenderer::renderGameOverShop(const Game& game, const MetaProgress& progress) {
    const Grid& grid = game.grid();
    int x = 12;
    int y = 40;

    // Essence balance — tint it with the theme's own "Fortune" (Score
    // bonus) color, since that's the closest thematic match already in the
    // palette.
    SDL_Color essenceColor = theme_->colorForEffect(EffectCategory::Score, EffectPolarity::Bonus);
    drawBlock(x, y, 22, essenceColor, *theme_, true);
    drawDigits(x + 34, y, progress.essence, 12, 26, 4, theme_->hudText);

    y += 50;
    constexpr std::array<UpgradeKind, 4> kinds{UpgradeKind::Vitality, UpgradeKind::Luck,
                                                UpgradeKind::Insurance,
                                                UpgradeKind::Regeneration};
    std::array<SDL_Color, 4> swatches{
        theme_->colorForEffect(EffectCategory::Life, EffectPolarity::Bonus),
        theme_->colorForEffect(EffectCategory::Vision, EffectPolarity::Bonus),
        theme_->shieldColor,
        theme_->colorForEffect(EffectCategory::Size, EffectPolarity::Bonus),
    };
    for (size_t i = 0; i < kinds.size(); ++i) {
        int rowY = y + static_cast<int>(i) * 40;
        drawBlock(x, rowY, 20, swatches[i], *theme_, false);
        drawDigits(x + 34, rowY - 3, progress.upgrades.level(kinds[i]), 8, 20, 3, theme_->hudText);
        long long cost = costForNextLevel(kinds[i], progress.upgrades.level(kinds[i]));
        drawDigits(x + 110, rowY - 3, cost, 8, 20, 3, swatches[i]);
    }

    (void)grid;
}

void GameRenderer::renderThemeMenu(const ThemeCatalog& catalog, size_t previewIndex) {
    int width = 0, height = 0;
    SDL_GetRendererOutputSize(renderer_, &width, &height);

    fillRect(0, 0, width, height, SDL_Color{0, 0, 0, 205});

    const auto& themes = catalog.themes();
    int count = static_cast<int>(themes.size());
    if (count == 0) return;
    int swatchSize = 64;
    int gap = 18;
    int totalWidth = count * swatchSize + (count - 1) * gap;
    int startX = (width - totalWidth) / 2;
    int y = height / 2 - swatchSize / 2 - 20;

    const Theme* originalTheme = theme_;
    for (int i = 0; i < count; ++i) {
        int x = startX + i * (swatchSize + gap);
        const Theme& swatchTheme = themes[static_cast<size_t>(i)];

        if (static_cast<size_t>(i) == previewIndex) {
            fillRect(x - 6, y - 6, swatchSize + 12, swatchSize + 12, SDL_Color{255, 255, 255, 255});
        }

        // Each swatch renders in *its own* block style/palette, live, so
        // Bevel3D themes visibly look different from Flat ones right here
        // in the picker rather than the player having to select-and-see.
        theme_ = &swatchTheme;
        drawBlock(x, y, swatchSize, swatchTheme.snakeHead, swatchTheme, true);
        theme_ = originalTheme;

        drawDigits(x + swatchSize / 2 - 6, y + swatchSize + 12, i + 1, 8, 16, 3,
                   originalTheme->hudText);
    }
}

void GameRenderer::render(const Game& game) {
    drawBoard(game);
    drawItems(game);
    drawSnake(game);
    drawVisionOverlay(game);
    drawHud(game);
    drawEffectBadges(game);
}

}  // namespace snake_engine

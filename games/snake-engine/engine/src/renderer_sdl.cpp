#include "snake_engine/renderer.hpp"

#include <algorithm>
#include <array>
#include <cmath>

namespace snake_engine {

namespace {

constexpr SDL_Color kBackground{18, 18, 24, 255};
constexpr SDL_Color kGridLine{32, 32, 40, 255};
constexpr SDL_Color kSnakeHead{120, 230, 140, 255};
constexpr SDL_Color kSnakeBody{70, 170, 100, 255};
constexpr SDL_Color kWhite{235, 235, 240, 255};
constexpr SDL_Color kHudBarBg{40, 40, 48, 255};

SDL_Color colorForEffect(EffectCategory category, EffectPolarity polarity) {
    bool bonus = polarity == EffectPolarity::Bonus;
    switch (category) {
        case EffectCategory::Life:
            return bonus ? SDL_Color{90, 220, 110, 255} : SDL_Color{220, 60, 60, 255};
        case EffectCategory::Speed:
            return bonus ? SDL_Color{240, 210, 70, 255} : SDL_Color{170, 130, 30, 255};
        case EffectCategory::Size:
            return bonus ? SDL_Color{170, 110, 230, 255} : SDL_Color{110, 60, 150, 255};
        case EffectCategory::Score:
            return bonus ? SDL_Color{250, 190, 60, 255} : SDL_Color{160, 100, 30, 255};
        case EffectCategory::Control:
            return bonus ? SDL_Color{80, 210, 220, 255} : SDL_Color{40, 130, 150, 255};
        case EffectCategory::Vision:
            return bonus ? SDL_Color{100, 150, 240, 255} : SDL_Color{60, 80, 150, 255};
        case EffectCategory::Shield:
            return SDL_Color{235, 235, 240, 255};
    }
    return kWhite;
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

GameRenderer::GameRenderer(SDL_Renderer* renderer) : renderer_(renderer) {}

void GameRenderer::fillRect(int x, int y, int w, int h, SDL_Color color) {
    SDL_SetRenderDrawColor(renderer_, color.r, color.g, color.b, color.a);
    SDL_Rect rect{x, y, w, h};
    SDL_RenderFillRect(renderer_, &rect);
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

void GameRenderer::drawBoard(const Game& game) {
    const Grid& grid = game.grid();
    SDL_SetRenderDrawColor(renderer_, kBackground.r, kBackground.g, kBackground.b,
                            kBackground.a);
    SDL_RenderClear(renderer_);

    SDL_SetRenderDrawColor(renderer_, kGridLine.r, kGridLine.g, kGridLine.b, kGridLine.a);
    for (int x = 0; x <= grid.width; ++x) {
        SDL_RenderDrawLine(renderer_, x * grid.cellSizePixels, 0, x * grid.cellSizePixels,
                            grid.heightPixels());
    }
    for (int y = 0; y <= grid.height; ++y) {
        SDL_RenderDrawLine(renderer_, 0, y * grid.cellSizePixels, grid.widthPixels(),
                            y * grid.cellSizePixels);
    }
}

void GameRenderer::drawSnake(const Game& game) {
    const Grid& grid = game.grid();
    const auto& body = game.snake().body();
    int inset = std::max(1, grid.cellSizePixels / 10);
    bool first = true;
    for (const auto& segment : body) {
        SDL_Color color = first ? kSnakeHead : kSnakeBody;
        fillRect(segment.x * grid.cellSizePixels + inset, segment.y * grid.cellSizePixels + inset,
                 grid.cellSizePixels - inset * 2, grid.cellSizePixels - inset * 2, color);
        first = false;
    }
}

void GameRenderer::drawItems(const Game& game) {
    const Grid& grid = game.grid();
    int inset = std::max(2, grid.cellSizePixels / 6);
    for (const auto& item : game.itemSpawner().items()) {
        SDL_Color color = colorForEffect(item.effect.category, item.effect.polarity);
        fillRect(item.position.x * grid.cellSizePixels + inset,
                 item.position.y * grid.cellSizePixels + inset,
                 grid.cellSizePixels - inset * 2, grid.cellSizePixels - inset * 2, color);
        if (item.effect.polarity == EffectPolarity::Drawback) {
            SDL_SetRenderDrawColor(renderer_, 0, 0, 0, 255);
            SDL_Rect outline{item.position.x * grid.cellSizePixels + inset,
                              item.position.y * grid.cellSizePixels + inset,
                              grid.cellSizePixels - inset * 2, grid.cellSizePixels - inset * 2};
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

    fillRect(barX, barY, barW, barH, kHudBarBg);
    float ratio = game.maxLife() > 0.0f ? game.life() / game.maxLife() : 0.0f;
    fillRect(barX, barY, static_cast<int>(barW * clamp(ratio, 0.0f, 1.0f)), barH,
             lifeBarColor(ratio));

    drawDigits(barX + barW + 16, barY - 3, static_cast<long long>(std::lround(game.life())), 10,
               24, 4, kWhite);

    drawDigits(barX, barY + barH + 14, static_cast<long long>(game.score()), 12, 28, 4, kWhite);

    for (int i = 0; i < game.shieldCharges(); ++i) {
        fillRect(barX + 260 + i * 20, barY, 14, 14, colorForEffect(EffectCategory::Shield,
                                                                     EffectPolarity::Bonus));
    }
}

void GameRenderer::drawEffectBadges(const Game& game) {
    const Grid& grid = game.grid();
    int x = 12;
    int y = grid.heightPixels() + 60;
    for (const auto& active : game.activeEffects()) {
        SDL_Color color = colorForEffect(active.effect.category, active.effect.polarity);
        fillRect(x, y, 14, 14, color);
        x += 22;
    }
}

void GameRenderer::renderGameOverShop(const Game& game, const MetaProgress& progress) {
    const Grid& grid = game.grid();
    int x = 12;
    int y = 40;

    // Essence balance.
    fillRect(x, y, 14, 26, SDL_Color{250, 210, 90, 255});
    drawDigits(x + 26, y, progress.essence, 12, 26, 4, kWhite);

    y += 50;
    constexpr std::array<UpgradeKind, 4> kinds{UpgradeKind::Vitality, UpgradeKind::Luck,
                                                UpgradeKind::Insurance,
                                                UpgradeKind::Regeneration};
    constexpr std::array<SDL_Color, 4> swatches{
        SDL_Color{90, 220, 110, 255},
        SDL_Color{80, 210, 220, 255},
        SDL_Color{235, 235, 240, 255},
        SDL_Color{170, 110, 230, 255},
    };
    for (size_t i = 0; i < kinds.size(); ++i) {
        int rowY = y + static_cast<int>(i) * 40;
        fillRect(x, rowY, 20, 20, swatches[i]);
        drawDigits(x + 34, rowY - 3, progress.upgrades.level(kinds[i]), 8, 20, 3, kWhite);
        long long cost = costForNextLevel(kinds[i], progress.upgrades.level(kinds[i]));
        drawDigits(x + 110, rowY - 3, cost, 8, 20, 3, swatches[i]);
    }

    (void)grid;
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

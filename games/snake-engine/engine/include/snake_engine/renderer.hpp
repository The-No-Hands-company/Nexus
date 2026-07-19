#pragma once

#include <SDL.h>

#include "snake_engine/game.hpp"
#include "snake_engine/theme.hpp"
#include "snake_engine/theme_catalog.hpp"
#include "snake_engine/upgrades.hpp"

namespace snake_engine {

// Draws one Game's worth of state into an existing SDL_Renderer. Text is
// rendered as simple seven-segment digits rather than pulling in SDL_ttf +
// a bundled font, which keeps the dependency footprint (and therefore the
// "builds cleanly everywhere" story) as small as possible.
//
// Every color and the block-drawing style itself come from the active
// Theme (see theme.hpp) rather than being hardcoded, so switching themes is
// just calling setTheme() — no rendering code changes per theme.
class GameRenderer {
public:
    explicit GameRenderer(SDL_Renderer* renderer);

    void setTheme(const Theme& theme) { theme_ = &theme; }
    [[nodiscard]] const Theme& theme() const { return *theme_; }

    void render(const Game& game);

    // Post-run upgrade shop: essence balance and the four upgrade levels,
    // laid out as colored key-swatches (1-4) with digit readouts.
    void renderGameOverShop(const Game& game, const MetaProgress& progress);

    // Full-screen theme picker: one swatch per theme, each drawn in *that*
    // theme's own block style so the visual difference is obvious before
    // committing to it. `previewIndex` is highlighted as the current pick.
    void renderThemeMenu(const ThemeCatalog& catalog, size_t previewIndex);

    void fillRect(int x, int y, int w, int h, SDL_Color color);
    void drawDigits(int x, int y, long long value, int digitWidth, int digitHeight,
                     int thickness, SDL_Color color);

private:
    SDL_Renderer* renderer_;
    const Theme* theme_;
    Theme defaultTheme_;

    void drawBoard(const Game& game);
    void drawSnake(const Game& game);
    void drawItems(const Game& game);
    void drawVisionOverlay(const Game& game);
    void drawHud(const Game& game);
    void drawEffectBadges(const Game& game);

    // Style-dispatching cell draw: Flat is a plain fillRect; Bevel3D adds a
    // gradient fill, raised-edge bevel, drop shadow, and specular highlight
    // using the currently active theme_ (or an explicitly passed one, for
    // rendering a swatch in a *different* theme's style than the game's).
    void drawBlock(int x, int y, int size, SDL_Color color, const Theme& style, bool glow);
    void drawBlockFlat(int x, int y, int size, SDL_Color color);
    void drawBlockBevel3D(int x, int y, int size, SDL_Color color, const Theme& style, bool glow);
    void fillGradientRectV(int x, int y, int w, int h, SDL_Color top, SDL_Color bottom);
    void drawBackground(int width, int height);
    void drawVignette(int width, int height);
};

}  // namespace snake_engine

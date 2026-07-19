#pragma once

#include <SDL.h>

#include "snake_engine/game.hpp"
#include "snake_engine/upgrades.hpp"

namespace snake_engine {

// Draws one Game's worth of state into an existing SDL_Renderer. Text is
// rendered as simple seven-segment digits rather than pulling in SDL_ttf +
// a bundled font, which keeps the dependency footprint (and therefore the
// "builds cleanly everywhere" story) as small as possible.
class GameRenderer {
public:
    explicit GameRenderer(SDL_Renderer* renderer);

    void render(const Game& game);

    // Post-run upgrade shop: essence balance and the four upgrade levels,
    // laid out as colored key-swatches (1-4) with digit readouts.
    void renderGameOverShop(const Game& game, const MetaProgress& progress);

    void fillRect(int x, int y, int w, int h, SDL_Color color);
    void drawDigits(int x, int y, long long value, int digitWidth, int digitHeight,
                     int thickness, SDL_Color color);

private:
    SDL_Renderer* renderer_;

    void drawBoard(const Game& game);
    void drawSnake(const Game& game);
    void drawItems(const Game& game);
    void drawVisionOverlay(const Game& game);
    void drawHud(const Game& game);
    void drawEffectBadges(const Game& game);
};

}  // namespace snake_engine

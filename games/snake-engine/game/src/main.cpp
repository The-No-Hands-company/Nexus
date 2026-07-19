#include <SDL.h>

#include <algorithm>
#include <chrono>
#include <iostream>
#include <string>

#include "snake_engine/effect_catalog.hpp"
#include "snake_engine/game.hpp"
#include "snake_engine/renderer.hpp"
#include "snake_engine/save.hpp"
#include "snake_engine/upgrades.hpp"
#include "snake_engine/window.hpp"

namespace {

constexpr int kGridWidth = 48;
constexpr int kGridHeight = 32;
constexpr int kCellSize = 18;
constexpr int kHudHeight = 130;

std::string basePath() {
    char* raw = SDL_GetBasePath();
    std::string path = raw != nullptr ? raw : "./";
    if (raw != nullptr) {
        SDL_free(raw);
    }
    return path;
}

}  // namespace

int main(int /*argc*/, char** /*argv*/) {
    using namespace snake_engine;

    const std::string base = basePath();
    const std::string dataPath = base + "data/effects/core_effects.json";
    const std::string savePath = base + "snake_engine_save.json";

    EffectCatalog catalog;
    if (!catalog.loadFromFile(dataPath)) {
        std::cerr << "SnakeEngine: no data file at " << dataPath
                  << " — using built-in defaults\n";
        catalog.loadBuiltinDefaults();
    }

    MetaProgress progress;
    loadMetaProgress(savePath, progress);  // ok if this fails on first run

    Grid grid{kGridWidth, kGridHeight, kCellSize};
    Window window(grid.widthPixels(), grid.heightPixels() + kHudHeight, "SnakeEngine");
    if (!window.ok()) {
        std::cerr << "SnakeEngine: failed to create window/renderer\n";
        return 1;
    }

    Game game(grid, catalog, computeModifiers(progress.upgrades));
    GameRenderer renderer(window.sdlRenderer());

    bool paused = false;
    bool essenceAwardedThisRun = false;
    InputState input;
    auto lastTime = std::chrono::steady_clock::now();

    while (true) {
        input.quitRequested = false;
        input.restartRequested = false;
        input.togglePause = false;
        window.pollEvents(input);
        if (input.quitRequested) {
            break;
        }
        if (input.togglePause) {
            paused = !paused;
        }

        auto now = std::chrono::steady_clock::now();
        float dt = std::chrono::duration<float>(now - lastTime).count();
        dt = std::min(dt, 0.1f);  // clamp huge stalls (e.g. window drag) to avoid a death spiral
        lastTime = now;

        if (game.status() == GameStatus::GameOver) {
            if (!essenceAwardedThisRun) {
                progress.essence += MetaProgress::essenceForScore(game.score());
                saveMetaProgress(savePath, progress);
                essenceAwardedThisRun = true;
            }
            // Number keys 1-4 buy upgrades between runs; R starts a new run.
            const Uint8* keys = SDL_GetKeyboardState(nullptr);
            static bool keyHeld[4] = {false, false, false, false};
            const UpgradeKind kinds[4] = {UpgradeKind::Vitality, UpgradeKind::Luck,
                                           UpgradeKind::Insurance, UpgradeKind::Regeneration};
            const SDL_Scancode scancodes[4] = {SDL_SCANCODE_1, SDL_SCANCODE_2, SDL_SCANCODE_3,
                                                SDL_SCANCODE_4};
            for (int i = 0; i < 4; ++i) {
                bool down = keys[scancodes[i]] != 0;
                if (down && !keyHeld[i]) {
                    progress.purchase(kinds[i]);
                    saveMetaProgress(savePath, progress);
                }
                keyHeld[i] = down;
            }

            if (input.restartRequested) {
                game.setModifiers(computeModifiers(progress.upgrades));
                game.reset();
                essenceAwardedThisRun = false;
            }
        } else if (!paused) {
            game.update(dt, input.cursorPixelPos);
        }

        renderer.render(game);
        if (game.status() == GameStatus::GameOver) {
            renderer.renderGameOverShop(game, progress);
        }
        window.present();
    }

    saveMetaProgress(savePath, progress);
    return 0;
}

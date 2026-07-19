#include <SDL.h>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

#include <algorithm>
#include <array>
#include <chrono>
#include <iostream>
#include <memory>
#include <string>

#include "snake_engine/effect_catalog.hpp"
#include "snake_engine/game.hpp"
#include "snake_engine/renderer.hpp"
#include "snake_engine/save.hpp"
#include "snake_engine/theme_catalog.hpp"
#include "snake_engine/upgrades.hpp"
#include "snake_engine/window.hpp"

using namespace snake_engine;

namespace {

constexpr int kGridWidth = 48;
constexpr int kGridHeight = 32;
constexpr int kDefaultCellSize = 18;
constexpr int kHudHeight = 130;

std::string basePath() {
    char* raw = SDL_GetBasePath();
    std::string path = raw != nullptr ? raw : "./";
    if (raw != nullptr) {
        SDL_free(raw);
    }
    return path;
}

// All state one running instance needs, heap-allocated once so a plain
// pointer can be handed to Emscripten's callback-driven main loop (which
// takes a C function pointer + void* userdata, not a capturing lambda).
struct App {
    std::string savePath;
    EffectCatalog catalog;
    ThemeCatalog themeCatalog;
    MetaProgress progress;
    std::unique_ptr<Window> window;
    std::unique_ptr<Game> game;
    std::unique_ptr<GameRenderer> renderer;
    bool paused = false;
    bool essenceAwardedThisRun = false;
    bool quit = false;
    InputState input;
    std::chrono::steady_clock::time_point lastTime;

    // Theme picker overlay (T to open). Navigating previews the theme live
    // against the running board; confirming persists it, Escape cancels
    // back to whatever was active before the menu was opened.
    bool themeMenuOpen = false;
    bool pausedBeforeMenu = false;
    size_t confirmedThemeIndex = 0;
    size_t previewThemeIndex = 0;
};

std::unique_ptr<App> g_app;

void persistSave(App& app) {
    saveMetaProgress(app.savePath, app.progress);
#ifdef __EMSCRIPTEN__
    // MEMFS writes are in-memory only; flush them out to IndexedDB so
    // essence/upgrades survive a page reload. Fire-and-forget: by the time
    // the player closes the tab this has almost always already landed.
    EM_ASM(
        FS.syncfs(false, function(err) {
            if (err) { console.error("SnakeEngine: IDBFS save sync failed", err); }
        });
    );
#endif
}

void applyPreviewTheme(App& app) {
    app.renderer->setTheme(app.themeCatalog.themeAt(app.previewThemeIndex));
}

// Handles input while the theme picker overlay is open: Left/Right or a
// direct number key changes the live preview, Enter or T again confirms
// and persists it, Escape cancels back to the previously-confirmed theme.
void handleThemeMenuInput(App& app) {
    size_t themeCount = app.themeCatalog.themes().size();
    if (themeCount == 0) {
        app.themeMenuOpen = false;
        return;
    }

    if (app.input.quitRequested) {
        app.previewThemeIndex = app.confirmedThemeIndex;
        applyPreviewTheme(app);
        app.themeMenuOpen = false;
        app.paused = app.pausedBeforeMenu;
        app.input.quitRequested = false;  // cancel the menu, not the whole game
        return;
    }

    if (app.input.navigateLeft) {
        app.previewThemeIndex = (app.previewThemeIndex + themeCount - 1) % themeCount;
        applyPreviewTheme(app);
    }
    if (app.input.navigateRight) {
        app.previewThemeIndex = (app.previewThemeIndex + 1) % themeCount;
        applyPreviewTheme(app);
    }

    constexpr std::array<SDL_Scancode, 9> kNumberScancodes{
        SDL_SCANCODE_1, SDL_SCANCODE_2, SDL_SCANCODE_3, SDL_SCANCODE_4, SDL_SCANCODE_5,
        SDL_SCANCODE_6, SDL_SCANCODE_7, SDL_SCANCODE_8, SDL_SCANCODE_9,
    };
    static std::array<bool, 9> keyHeld{};
    const Uint8* keys = SDL_GetKeyboardState(nullptr);
    for (size_t i = 0; i < kNumberScancodes.size() && i < themeCount; ++i) {
        bool down = keys[kNumberScancodes[i]] != 0;
        if (down && !keyHeld[i]) {
            app.previewThemeIndex = i;
            applyPreviewTheme(app);
        }
        keyHeld[i] = down;
    }

    if (app.input.confirm || app.input.toggleThemeMenu) {
        app.confirmedThemeIndex = app.previewThemeIndex;
        app.progress.themeId = app.themeCatalog.themeAt(app.confirmedThemeIndex).id;
        persistSave(app);
        app.themeMenuOpen = false;
        app.paused = app.pausedBeforeMenu;
    }
}

// One frame: input, simulation step, render. Shared by the native blocking
// loop and Emscripten's non-blocking emscripten_set_main_loop_arg callback.
void iterate(void* arg) {
    App& app = *static_cast<App*>(arg);

    app.input.quitRequested = false;
    app.input.restartRequested = false;
    app.input.togglePause = false;
    app.input.toggleThemeMenu = false;
    app.input.navigateLeft = false;
    app.input.navigateRight = false;
    app.input.confirm = false;
    app.window->pollEvents(app.input);

    if (app.themeMenuOpen) {
        handleThemeMenuInput(app);
        app.renderer->render(*app.game);
        if (app.themeMenuOpen) {
            app.renderer->renderThemeMenu(app.themeCatalog, app.previewThemeIndex);
        } else if (app.game->status() == GameStatus::GameOver) {
            app.renderer->renderGameOverShop(*app.game, app.progress);
        }
        app.window->present();
        return;
    }

    if (app.input.quitRequested) {
        app.quit = true;
        persistSave(app);
#ifdef __EMSCRIPTEN__
        emscripten_cancel_main_loop();
#endif
        return;
    }
    if (app.input.togglePause) {
        app.paused = !app.paused;
    }
    if (app.input.toggleThemeMenu) {
        app.themeMenuOpen = true;
        app.pausedBeforeMenu = app.paused;
        app.paused = true;
        app.previewThemeIndex = app.confirmedThemeIndex;
    }

    auto now = std::chrono::steady_clock::now();
    float dt = std::chrono::duration<float>(now - app.lastTime).count();
    dt = std::min(dt, 0.1f);  // clamp huge stalls (e.g. window drag) to avoid a death spiral
    app.lastTime = now;

    if (app.game->status() == GameStatus::GameOver) {
        if (!app.essenceAwardedThisRun) {
            app.progress.essence += MetaProgress::essenceForScore(app.game->score());
            persistSave(app);
            app.essenceAwardedThisRun = true;
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
                app.progress.purchase(kinds[i]);
                persistSave(app);
            }
            keyHeld[i] = down;
        }

        if (app.input.restartRequested) {
            app.game->setModifiers(computeModifiers(app.progress.upgrades));
            app.game->reset();
            app.essenceAwardedThisRun = false;
        }
    } else if (!app.paused) {
        app.game->update(dt, app.input.cursorPixelPos);
    }

    app.renderer->render(*app.game);
    if (app.game->status() == GameStatus::GameOver) {
        app.renderer->renderGameOverShop(*app.game, app.progress);
    }

    app.window->present();
}

// Builds the window/game/renderer and enters the main loop. Deferred behind
// a function (rather than living directly in main()) because on Emscripten
// it can't start until an async IndexedDB load finishes — see main() below.
void start() {
    g_app = std::make_unique<App>();
    App& app = *g_app;

    const std::string base = basePath();
    std::string dataPath;
    std::string themeDataPath;
#if defined(__EMSCRIPTEN__)
    // Populated by --preload-file (see game/CMakeLists.txt) into the
    // in-memory filesystem at exactly this path.
    dataPath = "/data/effects/core_effects.json";
    themeDataPath = "/data/themes/themes.json";
    app.savePath = "/save/snake_engine_save.json";
#elif defined(__ANDROID__)
    // SDL_RWFromFile falls back to the APK's assets/ for a path like this
    // one that doesn't resolve on the real filesystem — see
    // engine/src/effect_catalog.cpp's readWholeFile(). The android/app
    // Gradle module packages data/'s *contents* directly under assets/
    // (see android/app/build.gradle's assets.srcDirs), so there's no
    // "data/" prefix here unlike the other platforms.
    dataPath = "effects/core_effects.json";
    themeDataPath = "themes/themes.json";
    app.savePath = std::string(SDL_AndroidGetInternalStoragePath()) + "/snake_engine_save.json";
#else
    dataPath = base + "data/effects/core_effects.json";
    themeDataPath = base + "data/themes/themes.json";
    app.savePath = base + "snake_engine_save.json";
#endif

    if (!app.catalog.loadFromFile(dataPath)) {
        std::cerr << "SnakeEngine: no data file at " << dataPath
                  << " — using built-in defaults\n";
        app.catalog.loadBuiltinDefaults();
    }
    if (!app.themeCatalog.loadFromFile(themeDataPath)) {
        std::cerr << "SnakeEngine: no theme file at " << themeDataPath
                  << " — using built-in theme library\n";
        app.themeCatalog.loadBuiltinDefaults();
    }

    loadMetaProgress(app.savePath, app.progress);  // ok if this fails on first run
    app.confirmedThemeIndex = app.themeCatalog.indexForId(app.progress.themeId);
    app.previewThemeIndex = app.confirmedThemeIndex;

    app.window = std::make_unique<Window>(kGridWidth * kDefaultCellSize,
                                           kGridHeight * kDefaultCellSize + kHudHeight,
                                           "SnakeEngine");
    if (!app.window->ok()) {
        std::cerr << "SnakeEngine: failed to create window/renderer\n";
        g_app.reset();
        return;
    }

    // The window's actual pixel size may not match what was requested —
    // Android always hands back the full device display regardless of the
    // constructor arguments above, and the player may resize a desktop
    // window. Size grid cells to fit whatever we actually got.
    int outputWidth = 0;
    int outputHeight = 0;
    SDL_GetRendererOutputSize(app.window->sdlRenderer(), &outputWidth, &outputHeight);
    int cellSize = kDefaultCellSize;
    if (outputWidth > 0 && outputHeight > 0) {
        int cellFromWidth = outputWidth / kGridWidth;
        int cellFromHeight = std::max(1, outputHeight - kHudHeight) / kGridHeight;
        cellSize = std::max(4, std::min(cellFromWidth, cellFromHeight));
    }
    Grid grid{kGridWidth, kGridHeight, cellSize};

    app.game = std::make_unique<Game>(grid, app.catalog, computeModifiers(app.progress.upgrades));
    app.renderer = std::make_unique<GameRenderer>(app.window->sdlRenderer());
    app.renderer->setTheme(app.themeCatalog.themeAt(app.confirmedThemeIndex));
    app.lastTime = std::chrono::steady_clock::now();

#ifdef __EMSCRIPTEN__
    emscripten_set_main_loop_arg(iterate, &app, 0, true);
#else
    while (!app.quit) {
        iterate(&app);
    }
    g_app.reset();
#endif
}

}  // namespace

#ifdef __EMSCRIPTEN__
extern "C" void EMSCRIPTEN_KEEPALIVE snake_engine_start() {
    start();
}
#endif

// No-argument signature deliberately: argc/argv are never used, and a
// wasm32 build of `main(int, char**)` compiles to a differently-named
// `__main_argc_argv` symbol that (at least with this toolchain/link flag
// combination) never gets its `main` trampoline linked in, silently
// leaving the program without a callable entry point.
int main() {
#ifdef __EMSCRIPTEN__
    // Mount IndexedDB-backed storage for the save file and load whatever is
    // already there before the game (and therefore loadMetaProgress) starts
    // — FS.syncfs is asynchronous, so start() has to run from its callback
    // rather than right after main() returns. -sEXIT_RUNTIME=0 (see
    // game/CMakeLists.txt) keeps the wasm runtime alive in the meantime.
    EM_ASM(
        try {
            FS.mkdir('/save');
            FS.mount(IDBFS, {}, '/save');
            FS.syncfs(true, function(err) {
                if (err) { console.error("SnakeEngine: IDBFS initial sync failed", err); }
                ccall('snake_engine_start', null, [], []);
            });
        } catch (e) {
            console.error("SnakeEngine: IDBFS mount failed, starting without persistence", e);
            ccall('snake_engine_start', null, [], []);
        }
    );
#else
    start();
#endif
    return 0;
}

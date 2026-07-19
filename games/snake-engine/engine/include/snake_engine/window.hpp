#pragma once

#include <string>

#include "snake_engine/math.hpp"

struct SDL_Window;
struct SDL_Renderer;

namespace snake_engine {

struct InputState {
    Vec2f cursorPixelPos{0.0f, 0.0f};
    bool quitRequested = false;
    bool restartRequested = false;
    bool togglePause = false;
};

// Thin SDL2 wrapper. This is the only place window/event-loop platform
// details live, so a future backend (or the editor's embedded viewport)
// only has to replace this one class.
class Window {
public:
    Window(int width, int height, const std::string& title);
    ~Window();
    Window(const Window&) = delete;
    Window& operator=(const Window&) = delete;

    [[nodiscard]] bool ok() const { return window_ != nullptr && renderer_ != nullptr; }

    // Merges newly-arrived SDL events into `input`. Cursor position persists
    // between calls; quit/restart/pause flags are reset by the caller.
    void pollEvents(InputState& input);
    void present();

    [[nodiscard]] SDL_Renderer* sdlRenderer() const { return renderer_; }

private:
    SDL_Window* window_ = nullptr;
    SDL_Renderer* renderer_ = nullptr;
};

}  // namespace snake_engine

#include "snake_engine/window.hpp"

#include <SDL.h>

namespace snake_engine {

Window::Window(int width, int height, const std::string& title) {
    if (SDL_WasInit(SDL_INIT_VIDEO) == 0) {
        SDL_Init(SDL_INIT_VIDEO);
    }

    window_ = SDL_CreateWindow(title.c_str(), SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED,
                                width, height, SDL_WINDOW_SHOWN);
    if (window_ == nullptr) {
        return;
    }

    renderer_ = SDL_CreateRenderer(
        window_, -1, SDL_RENDERER_ACCELERATED | SDL_RENDERER_PRESENTVSYNC);
    if (renderer_ == nullptr) {
        renderer_ = SDL_CreateRenderer(window_, -1, SDL_RENDERER_SOFTWARE);
    }
}

Window::~Window() {
    if (renderer_ != nullptr) {
        SDL_DestroyRenderer(renderer_);
    }
    if (window_ != nullptr) {
        SDL_DestroyWindow(window_);
    }
    SDL_Quit();
}

void Window::pollEvents(InputState& input) {
    SDL_Event event;
    while (SDL_PollEvent(&event) != 0) {
        switch (event.type) {
            case SDL_QUIT:
                input.quitRequested = true;
                break;
            case SDL_MOUSEMOTION:
                input.cursorPixelPos = Vec2f{static_cast<float>(event.motion.x),
                                              static_cast<float>(event.motion.y)};
                break;
            case SDL_KEYDOWN:
                if (event.key.keysym.sym == SDLK_ESCAPE) {
                    input.quitRequested = true;
                } else if (event.key.keysym.sym == SDLK_r) {
                    input.restartRequested = true;
                } else if (event.key.keysym.sym == SDLK_p ||
                           event.key.keysym.sym == SDLK_SPACE) {
                    input.togglePause = true;
                } else if (event.key.keysym.sym == SDLK_t) {
                    input.toggleThemeMenu = true;
                } else if (event.key.keysym.sym == SDLK_LEFT) {
                    input.navigateLeft = true;
                } else if (event.key.keysym.sym == SDLK_RIGHT) {
                    input.navigateRight = true;
                } else if (event.key.keysym.sym == SDLK_RETURN ||
                           event.key.keysym.sym == SDLK_KP_ENTER) {
                    input.confirm = true;
                }
                break;
            default:
                break;
        }
    }
}

void Window::present() {
    SDL_RenderPresent(renderer_);
}

}  // namespace snake_engine

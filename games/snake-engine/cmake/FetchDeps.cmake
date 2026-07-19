# Dependency resolution for SnakeEngine.
#
# Strategy: prefer a system-installed package (fast, matches OS packaging
# conventions on Linux distros / Homebrew / vcpkg), and transparently fall
# back to fetching source via FetchContent when nothing is found. This is
# what lets a clean checkout build with nothing but a C++20 compiler and
# CMake on Windows/macOS/Linux.

include(FetchContent)

# ---------------------------------------------------------------------------
# SDL2 — window, input, 2D rendering backend
# ---------------------------------------------------------------------------
find_package(SDL2 QUIET)
if(NOT SDL2_FOUND)
    message(STATUS "SnakeEngine: system SDL2 not found, fetching source via FetchContent")
    set(SDL_TEST OFF CACHE BOOL "" FORCE)
    set(SDL_TESTS OFF CACHE BOOL "" FORCE)
    set(SDL_SHARED ON CACHE BOOL "" FORCE)
    set(SDL_STATIC OFF CACHE BOOL "" FORCE)
    FetchContent_Declare(
        SDL2
        GIT_REPOSITORY https://github.com/libsdl-org/SDL.git
        GIT_TAG release-2.30.9
        GIT_SHALLOW TRUE
    )
    FetchContent_MakeAvailable(SDL2)
endif()

# ---------------------------------------------------------------------------
# nlohmann/json — data-driven effect/item/upgrade definitions (JSON)
# ---------------------------------------------------------------------------
find_package(nlohmann_json QUIET)
if(NOT nlohmann_json_FOUND)
    message(STATUS "SnakeEngine: system nlohmann_json not found, fetching source via FetchContent")
    set(JSON_BuildTests OFF CACHE INTERNAL "")
    FetchContent_Declare(
        nlohmann_json
        GIT_REPOSITORY https://github.com/nlohmann/json.git
        GIT_TAG v3.11.3
        GIT_SHALLOW TRUE
    )
    FetchContent_MakeAvailable(nlohmann_json)
endif()

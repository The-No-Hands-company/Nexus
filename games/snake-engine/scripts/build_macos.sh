#!/usr/bin/env bash
# Build SnakeEngine on macOS.
#
# A system SDL2 + nlohmann-json (via Homebrew) speed the build up but are
# not required — CMake falls back to fetching both from source (see
# cmake/FetchDeps.cmake) if it can't find them.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if command -v brew >/dev/null 2>&1 && [ "${SNAKE_SKIP_BREW:-0}" != "1" ]; then
    echo "==> (optional) installing SDL2 + nlohmann-json via Homebrew"
    brew install cmake ninja sdl2 nlohmann-json || \
        echo "    brew install failed or was skipped — CMake will fetch dependencies from source instead"
fi

cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build -j"$(sysctl -n hw.ncpu)"

echo "==> Build complete: build/game/snake_game"

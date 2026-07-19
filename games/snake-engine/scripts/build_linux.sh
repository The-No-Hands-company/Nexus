#!/usr/bin/env bash
# Build SnakeEngine on Linux.
#
# A system SDL2 + nlohmann-json speed the build up but are not required —
# CMake falls back to fetching both from source (see cmake/FetchDeps.cmake)
# if it can't find them, so this script works on a bare distro too.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if command -v apt-get >/dev/null 2>&1 && [ "${SNAKE_SKIP_APT:-0}" != "1" ]; then
    echo "==> (optional) installing SDL2 + nlohmann-json dev packages via apt"
    sudo apt-get update
    sudo apt-get install -y build-essential cmake ninja-build libsdl2-dev nlohmann-json3-dev || \
        echo "    apt install failed or was skipped — CMake will fetch dependencies from source instead"
fi

cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build -j"$(nproc)"

echo "==> Build complete: build/game/snake_game"

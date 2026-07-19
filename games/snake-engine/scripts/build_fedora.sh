#!/usr/bin/env bash
# Build SnakeEngine on Fedora.
#
# A system SDL2 + nlohmann-json speed the build up but are not required —
# CMake falls back to fetching both from source (see cmake/FetchDeps.cmake)
# if it can't find them, so this script works on a bare Fedora install too.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if command -v dnf >/dev/null 2>&1 && [ "${SNAKE_SKIP_DNF:-0}" != "1" ]; then
    echo "==> (optional) installing SDL2 + nlohmann-json dev packages via dnf"
    sudo dnf install -y gcc-c++ cmake ninja-build SDL2-devel json-devel || \
        echo "    dnf install failed or was skipped — CMake will fetch dependencies from source instead"
fi

cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build -j"$(nproc)"

echo "==> Build complete: build/game/snake_game"

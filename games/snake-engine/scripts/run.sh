#!/usr/bin/env bash
# Build (if needed) and run the game on Linux/macOS.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if [ ! -d build ]; then
    cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo
fi
cmake --build build -j"$(command -v nproc >/dev/null && nproc || sysctl -n hw.ncpu)"

exec ./build/game/snake_game

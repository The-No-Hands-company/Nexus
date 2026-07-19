#!/usr/bin/env bash
# Build the WebAssembly/HTML version of SnakeEngine.
#
# Needs an Emscripten toolchain (emcc/emcmake) on PATH. The officially
# recommended way to get one:
#   git clone https://github.com/emscripten-core/emsdk.git
#   cd emsdk && ./emsdk install latest && ./emsdk activate latest
#   source ./emsdk_env.sh
#
# Note for Debian/Ubuntu users who instead `apt install emscripten`: that
# package ships an old Emscripten (3.1.6) configured with FROZEN_CACHE=true,
# which refuses to fetch/build the SDL2 port it needs on first use, and its
# pinned SDL2 port hash is currently stale against GitHub's regenerated
# archive zip. If you hit "Attempt to lock the cache but FROZEN_CACHE is
# set" or "Unexpected hash" errors, either switch to emsdk above, or work
# around it: give yourself a writable, unfrozen cache config
# (`FROZEN_CACHE = False` in a copy of /usr/share/emscripten/.emscripten,
# pointed to via EM_CONFIG) and set EMCC_LOCAL_PORTS=sdl2=<path to a
# release-2.0.20 SDL2 checkout> to bypass the hash check.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if ! command -v emcmake >/dev/null 2>&1; then
    echo "error: emcmake not found on PATH. Install/activate Emscripten first (see comments in this script)." >&2
    exit 1
fi

emcmake cmake -S . -B build-web -DCMAKE_BUILD_TYPE=Release
cmake --build build-web -j"$(nproc 2>/dev/null || sysctl -n hw.ncpu)"

echo "==> Build complete: build-web/game/{index.html,snake_game.js,snake_game.wasm,snake_game.data}"
echo "==> Serve it locally, e.g.: python3 -m http.server -d build-web/game 8080"

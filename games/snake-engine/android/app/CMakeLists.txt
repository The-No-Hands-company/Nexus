cmake_minimum_required(VERSION 3.20)
project(SnakeEngineAndroid LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(SNAKE_ENGINE_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../..")

# Reuses the exact same dependency resolution as the desktop/web builds.
# There is no system SDL2 package on Android, so this always takes the
# FetchContent branch and cross-compiles SDL2 for the target ABI via the
# NDK toolchain that the Android Gradle Plugin already configured CMake with.
list(APPEND CMAKE_MODULE_PATH "${SNAKE_ENGINE_ROOT}/cmake")
include(FetchDeps)

add_subdirectory("${SNAKE_ENGINE_ROOT}/engine" snake_engine_build)

# SDLActivity (org.libsdl.app, vendored under src/main/java) loads
# "libSDL2.so" then "libmain.so" and expects the latter to export SDL_main —
# which game/src/main.cpp already provides via SDL.h's `#define main
# SDL_main` on Android, so it is reused completely unmodified here.
add_library(main SHARED "${SNAKE_ENGINE_ROOT}/game/src/main.cpp")
target_link_libraries(main PRIVATE snake_engine SDL2::SDL2 SDL2::SDL2main)
target_compile_features(main PRIVATE cxx_std_20)

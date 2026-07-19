cmake_minimum_required(VERSION 3.20)
project(SnakeEngine
    VERSION 0.1.0
    DESCRIPTION "SnakeEngine — a cursor-driven snake game and the seed of a full 2D level/character/environment editor"
    LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE RelWithDebInfo CACHE STRING "Build type" FORCE)
endif()

option(SNAKE_BUILD_GAME "Build the playable game executable" ON)
option(SNAKE_BUILD_TESTS "Build the engine unit tests" ON)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")
include(FetchDeps)

add_subdirectory(engine)

if(SNAKE_BUILD_GAME)
    add_subdirectory(game)
endif()

if(SNAKE_BUILD_TESTS AND NOT EMSCRIPTEN)
    # The test binary exercises pure engine logic (RNG, effects, grid/snake
    # math) with no rendering — nothing about it is platform-specific, so
    # it's only built for the native desktop toolchain that's actually
    # running the build, rather than also standing up a Node-based ctest
    # runner for the wasm output.
    enable_testing()
    add_subdirectory(tests)
endif()

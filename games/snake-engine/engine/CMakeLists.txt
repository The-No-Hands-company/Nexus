add_library(snake_engine STATIC
    src/rng.cpp
    src/snake.cpp
    src/effect.cpp
    src/effect_catalog.cpp
    src/item.cpp
    src/upgrades.cpp
    src/save.cpp
    src/game.cpp
    src/window_sdl.cpp
    src/renderer_sdl.cpp
)

target_include_directories(snake_engine
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(snake_engine
    PUBLIC
        SDL2::SDL2
        nlohmann_json::nlohmann_json
)

target_compile_features(snake_engine PUBLIC cxx_std_20)

if(MSVC)
    target_compile_options(snake_engine PRIVATE /W4)
else()
    target_compile_options(snake_engine PRIVATE -Wall -Wextra -Wpedantic)
endif()

add_executable(snake_game src/main.cpp)

target_link_libraries(snake_game PRIVATE snake_engine)

if(WIN32)
    target_link_libraries(snake_game PRIVATE SDL2::SDL2main)
endif()

target_compile_features(snake_game PRIVATE cxx_std_20)

if(MSVC)
    target_compile_options(snake_game PRIVATE /W4)
elseif(NOT EMSCRIPTEN)
    target_compile_options(snake_game PRIVATE -Wall -Wextra -Wpedantic)
endif()

if(EMSCRIPTEN)
    # -sEXIT_RUNTIME=0: main() returns immediately after kicking off the
    #   async IDBFS load (see game/src/main.cpp); the runtime must stay
    #   alive so the JS callback can still call back into wasm afterward.
    # --preload-file: bundles data/ into the virtual filesystem at "/data",
    #   matching the "/data/..." path main.cpp uses for __EMSCRIPTEN__.
    #
    # Output is plain .js/.wasm, not emcc's auto-generated .html: web/index.html
    # (copied alongside below) is a small hand-written shell instead, which
    # sidesteps emcc's optional HTML-minify post-processing step entirely
    # and lets the page carry SnakeEngine's own title/styling.
    set_target_properties(snake_game PROPERTIES SUFFIX ".js")
    target_link_options(snake_game PRIVATE
        "-sALLOW_MEMORY_GROWTH=1"
        "-sEXIT_RUNTIME=0"
        "-lidbfs.js"
        "--preload-file=${CMAKE_SOURCE_DIR}/data@/data"
    )
    add_custom_command(TARGET snake_game POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy
            ${CMAKE_CURRENT_SOURCE_DIR}/web/index.html $<TARGET_FILE_DIR:snake_game>/index.html
    )
else()
    # Ship the data-driven effect definitions next to the built binary so a
    # fresh `cmake --build .` is immediately runnable without any install step.
    add_custom_command(TARGET snake_game POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_directory
            ${CMAKE_SOURCE_DIR}/data $<TARGET_FILE_DIR:snake_game>/data
    )
endif()

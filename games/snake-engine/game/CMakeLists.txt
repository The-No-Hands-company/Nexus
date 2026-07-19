add_executable(snake_game src/main.cpp)

target_link_libraries(snake_game PRIVATE snake_engine)

if(WIN32)
    target_link_libraries(snake_game PRIVATE SDL2::SDL2main)
endif()

target_compile_features(snake_game PRIVATE cxx_std_20)

if(MSVC)
    target_compile_options(snake_game PRIVATE /W4)
else()
    target_compile_options(snake_game PRIVATE -Wall -Wextra -Wpedantic)
endif()

# Ship the data-driven effect definitions next to the built binary so a
# fresh `cmake --build .` is immediately runnable without any install step.
add_custom_command(TARGET snake_game POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        ${CMAKE_SOURCE_DIR}/data $<TARGET_FILE_DIR:snake_game>/data
)

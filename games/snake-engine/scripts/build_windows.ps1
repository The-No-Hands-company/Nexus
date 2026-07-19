# Build SnakeEngine on Windows.
#
# No system SDL2/nlohmann-json is assumed: CMake's FetchContent fallback
# (cmake/FetchDeps.cmake) pulls both from source automatically, so this
# works from a clean checkout with just Visual Studio (or the standalone
# Build Tools) and CMake installed.
#
# Usage (from a "Developer PowerShell for VS" prompt):
#   powershell -ExecutionPolicy Bypass -File scripts\build_windows.ps1

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo
cmake --build build --config RelWithDebInfo --parallel

Write-Host "==> Build complete: build\game\RelWithDebInfo\snake_game.exe"

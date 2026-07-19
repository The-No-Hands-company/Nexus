# SnakeEngine — Android

A thin Gradle/NDK wrapper around the same `engine/` + `game/src/main.cpp`
everything else builds — see `app/CMakeLists.txt`, which `add_subdirectory`s
`../../engine` and reuses `../../cmake/FetchDeps.cmake` unchanged (there's no
system SDL2 package on Android, so it always cross-compiles SDL2 from source
via the NDK toolchain the Android Gradle Plugin configures CMake with).

`app/src/main/java/org/libsdl/app/` is SDL2's own Java glue
(`SDLActivity` and friends), vendored unmodified from the
[SDL 2.30.9 android-project template](https://github.com/libsdl-org/SDL/tree/release-2.30.9/android-project)
under its zlib license (`SDL2-LICENSE.txt` in that same directory).
`MainActivity` is a one-line subclass so the app doesn't ship under
`org.libsdl.app`'s own package name.

## Building

Requires an Android SDK + NDK (26.x) and Gradle. Easiest path is opening
`android/` as a project in Android Studio and letting it prompt for any
missing SDK/NDK components. From the command line:

```bash
cd android
./gradlew assembleDebug          # -> app/build/outputs/apk/debug/app-debug.apk
./gradlew installDebug           # requires a connected device/emulator
```

`local.properties` (gitignored, machine-specific) needs an `sdk.dir=`
pointing at your SDK if Gradle can't find it via `ANDROID_HOME`.

Native code is built for `arm64-v8a`, `armeabi-v7a`, and `x86_64`
(`app/build.gradle`'s `abiFilters`) — restrict that list for faster local
iteration, e.g. `./gradlew assembleDebug -Pandroid.injected.build.abi=arm64-v8a`.

## Controls

The mouse-cursor steering everywhere else becomes touch: SDL2 synthesizes
mouse-motion events from touch input on Android by default, so
`Snake::steerToward` needs no Android-specific code — drag a finger and the
snake follows it the same way a cursor would on desktop.

## Data & saves

- `data/effects/core_effects.json` is packaged into the APK's `assets/`
  (`app/build.gradle`'s `assets.srcDirs`) and loaded through `SDL_RWFromFile`,
  which transparently falls back to `AssetManager` for a path that isn't on
  the real filesystem — see `engine/src/effect_catalog.cpp`.
- The save file lives in the app's private internal storage
  (`SDL_AndroidGetInternalStoragePath()`), not alongside the APK.

## What's verified vs. not

This has been built end-to-end in CI-equivalent conditions (Gradle +
NDK r26c + CMake 3.22, all three ABIs) into a valid, correctly-signed debug
APK with the expected `libSDL2.so`/`libmain.so`/`libc++_shared.so` per ABI
and the JSON asset in the right place — confirmed with `aapt dump badging`
and by inspecting the APK contents directly. It has **not** been run on a
real device or emulator (no hardware-accelerated emulator was available in
the environment this was built in) — if you hit an on-device issue building
or running it, please file it.

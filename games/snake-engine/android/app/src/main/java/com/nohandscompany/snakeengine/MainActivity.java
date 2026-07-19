package com.nohandscompany.snakeengine;

import org.libsdl.app.SDLActivity;

// SDLActivity already does everything needed: loads libSDL2.so then
// libmain.so (built by ../CMakeLists.txt from engine/ + game/src/main.cpp)
// and calls that library's SDL_main. Subclassed only so the manifest/app
// have a package-local activity name instead of org.libsdl.app's own.
public class MainActivity extends SDLActivity {
}

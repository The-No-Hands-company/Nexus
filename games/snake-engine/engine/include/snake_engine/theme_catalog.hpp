#pragma once

#include <string>
#include <vector>

#include "snake_engine/theme.hpp"

namespace snake_engine {

// Holds the library of available themes. loadFromFile() reads
// data/themes/themes.json; loadBuiltinDefaults() mirrors the same themes in
// code so the game still has a full theme library if that file is missing.
class ThemeCatalog {
public:
    bool loadFromFile(const std::string& path);
    void loadBuiltinDefaults();

    [[nodiscard]] const std::vector<Theme>& themes() const { return themes_; }

    // Index is clamped into range, so callers never have to bounds-check
    // after a cycle/wrap computation.
    [[nodiscard]] const Theme& themeAt(size_t index) const;

    // Falls back to index 0 (the first theme, conventionally "Classic") if
    // no theme with this id exists — e.g. a save file referencing a theme
    // id that a data file edit later removed.
    [[nodiscard]] size_t indexForId(const std::string& id) const;

private:
    std::vector<Theme> themes_;
};

}  // namespace snake_engine

#include "snake_engine/theme_catalog.hpp"

#include <SDL.h>

#include <algorithm>
#include <cmath>
#include <nlohmann/json.hpp>

#include "snake_engine/math.hpp"

namespace snake_engine {

namespace {

using json = nlohmann::json;

// --- color parsing --------------------------------------------------------

Uint8 hexPair(const std::string& s, size_t offset) {
    return static_cast<Uint8>(std::stoul(s.substr(offset, 2), nullptr, 16));
}

// Accepts "#RRGGBB" or "#RRGGBBAA"; anything malformed falls back to opaque
// black rather than throwing, so a typo in a theme file degrades instead of
// crashing the game.
SDL_Color colorFromHex(const std::string& hex) {
    if (hex.size() != 7 && hex.size() != 9) {
        return SDL_Color{0, 0, 0, 255};
    }
    if (hex[0] != '#') {
        return SDL_Color{0, 0, 0, 255};
    }
    SDL_Color color{hexPair(hex, 1), hexPair(hex, 3), hexPair(hex, 5), 255};
    if (hex.size() == 9) {
        color.a = hexPair(hex, 7);
    }
    return color;
}

SDL_Color colorFromJson(const json& j, const std::string& key, SDL_Color fallback) {
    if (!j.contains(key) || !j[key].is_string()) {
        return fallback;
    }
    return colorFromHex(j[key].get<std::string>());
}

// --- HSV-based palette generation ------------------------------------------
// Lets a theme author supply one "accent" color and get a full, distinct set
// of per-category bonus/drawback colors for free, instead of hand-picking 14
// colors per theme. Bonus tiers sit at high value/saturation around the
// color wheel; drawback tiers are the same hues pulled darker and warped
// toward red, so "this is bad" reads as a color shift even before the
// player learns the category icons.

struct Hsv {
    float h, s, v;
};

Hsv rgbToHsv(SDL_Color c) {
    float r = c.r / 255.0f, g = c.g / 255.0f, b = c.b / 255.0f;
    float maxV = std::max({r, g, b});
    float minV = std::min({r, g, b});
    float delta = maxV - minV;
    float h = 0.0f;
    if (delta > 0.00001f) {
        if (maxV == r) {
            h = std::fmod((g - b) / delta, 6.0f);
        } else if (maxV == g) {
            h = (b - r) / delta + 2.0f;
        } else {
            h = (r - g) / delta + 4.0f;
        }
        h *= 60.0f;
        if (h < 0.0f) h += 360.0f;
    }
    float s = maxV <= 0.0f ? 0.0f : delta / maxV;
    return Hsv{h, s, maxV};
}

SDL_Color hsvToColor(Hsv hsv, Uint8 alpha) {
    float c = hsv.v * hsv.s;
    float hp = std::fmod(hsv.h, 360.0f) / 60.0f;
    float x = c * (1.0f - std::fabs(std::fmod(hp, 2.0f) - 1.0f));
    float r = 0, g = 0, b = 0;
    if (hp < 1) {
        r = c; g = x;
    } else if (hp < 2) {
        r = x; g = c;
    } else if (hp < 3) {
        g = c; b = x;
    } else if (hp < 4) {
        g = x; b = c;
    } else if (hp < 5) {
        r = x; b = c;
    } else {
        r = c; b = x;
    }
    float m = hsv.v - c;
    auto toByte = [](float v) { return static_cast<Uint8>(clamp(v, 0.0f, 1.0f) * 255.0f); };
    return SDL_Color{toByte(r + m), toByte(g + m), toByte(b + m), alpha};
}

constexpr int kCategoryCount = 7;

std::array<SDL_Color, kCategoryCount> deriveBonusColors(SDL_Color accent) {
    Hsv base = rgbToHsv(accent);
    std::array<SDL_Color, kCategoryCount> result{};
    for (int i = 0; i < kCategoryCount; ++i) {
        float hue = base.h + static_cast<float>(i) * (360.0f / kCategoryCount);
        result[static_cast<size_t>(i)] = hsvToColor(Hsv{hue, 0.72f, 0.95f}, 255);
    }
    return result;
}

std::array<SDL_Color, kCategoryCount> deriveDrawbackColors(SDL_Color accent) {
    Hsv base = rgbToHsv(accent);
    constexpr float kDangerHue = 6.0f;  // near-red
    std::array<SDL_Color, kCategoryCount> result{};
    for (int i = 0; i < kCategoryCount; ++i) {
        float hue = base.h + static_cast<float>(i) * (360.0f / kCategoryCount);
        // Blend 45% of the way toward red so every drawback still reads as
        // "danger" while keeping enough hue separation to tell categories
        // apart at a glance.
        float shortestDelta = std::fmod((kDangerHue - hue) + 540.0f, 360.0f) - 180.0f;
        float blendedHue = hue + shortestDelta * 0.45f;
        result[static_cast<size_t>(i)] = hsvToColor(Hsv{blendedHue, 0.80f, 0.72f}, 255);
    }
    return result;
}

BlockStyle blockStyleFromString(const std::string& s) {
    return s == "bevel3d" ? BlockStyle::Bevel3D : BlockStyle::Flat;
}

BackgroundStyle backgroundStyleFromString(const std::string& s) {
    return s == "vertical_gradient" ? BackgroundStyle::VerticalGradient : BackgroundStyle::Solid;
}

}  // namespace

bool ThemeCatalog::loadFromFile(const std::string& path) {
    SDL_RWops* rw = SDL_RWFromFile(path.c_str(), "rb");
    if (rw == nullptr) {
        return false;
    }
    Sint64 size = SDL_RWsize(rw);
    if (size <= 0) {
        SDL_RWclose(rw);
        return false;
    }
    std::string contents;
    contents.resize(static_cast<size_t>(size));
    size_t readBytes = SDL_RWread(rw, contents.data(), 1, static_cast<size_t>(size));
    SDL_RWclose(rw);
    if (readBytes != static_cast<size_t>(size)) {
        return false;
    }

    json root;
    try {
        root = json::parse(contents);
    } catch (const json::parse_error&) {
        return false;
    }
    if (!root.contains("themes") || !root["themes"].is_array()) {
        return false;
    }

    std::vector<Theme> parsed;
    for (const auto& t : root["themes"]) {
        Theme theme;
        theme.id = t.value("id", "unknown");
        theme.displayName = t.value("display_name", theme.id);
        theme.blockStyle = blockStyleFromString(t.value("block_style", "flat"));
        theme.backgroundStyle = backgroundStyleFromString(t.value("background_style", "solid"));

        theme.backgroundTop = colorFromJson(t, "background_top", theme.backgroundTop);
        theme.backgroundBottom = colorFromJson(t, "background_bottom", theme.backgroundTop);
        theme.gridLine = colorFromJson(t, "grid_line", theme.gridLine);
        theme.snakeHead = colorFromJson(t, "snake_head", theme.snakeHead);
        theme.snakeBody = colorFromJson(t, "snake_body", theme.snakeBody);
        theme.hudBarBackground = colorFromJson(t, "hud_bar_background", theme.hudBarBackground);
        theme.hudText = colorFromJson(t, "hud_text", theme.hudText);
        theme.vignette = colorFromJson(t, "vignette", theme.vignette);
        theme.shieldColor = colorFromJson(t, "shield_color", theme.hudText);
        theme.glowIntensity = t.value("glow_intensity", 0.0f);

        if (t.value("derive_categories_from_accent", false)) {
            SDL_Color accent = colorFromJson(t, "accent", theme.snakeHead);
            theme.bonusColors = deriveBonusColors(accent);
            theme.drawbackColors = deriveDrawbackColors(accent);
        } else if (t.contains("bonus_colors") && t.contains("drawback_colors") &&
                   t["bonus_colors"].is_array() && t["drawback_colors"].is_array() &&
                   t["bonus_colors"].size() == static_cast<size_t>(kCategoryCount) &&
                   t["drawback_colors"].size() == static_cast<size_t>(kCategoryCount)) {
            for (size_t i = 0; i < static_cast<size_t>(kCategoryCount); ++i) {
                theme.bonusColors[i] = colorFromHex(t["bonus_colors"][i].get<std::string>());
                theme.drawbackColors[i] = colorFromHex(t["drawback_colors"][i].get<std::string>());
            }
        } else {
            theme.bonusColors = deriveBonusColors(theme.snakeHead);
            theme.drawbackColors = deriveDrawbackColors(theme.snakeHead);
        }

        parsed.push_back(std::move(theme));
    }

    if (parsed.empty()) {
        return false;
    }
    themes_ = std::move(parsed);
    return true;
}

void ThemeCatalog::loadBuiltinDefaults() {
    themes_.clear();

    Theme classic;
    classic.id = "classic";
    classic.displayName = "Classic";
    classic.blockStyle = BlockStyle::Flat;
    classic.backgroundStyle = BackgroundStyle::Solid;
    classic.backgroundTop = classic.backgroundBottom = SDL_Color{18, 18, 24, 255};
    classic.gridLine = SDL_Color{32, 32, 40, 255};
    classic.snakeHead = SDL_Color{120, 230, 140, 255};
    classic.snakeBody = SDL_Color{70, 170, 100, 255};
    classic.hudBarBackground = SDL_Color{40, 40, 48, 255};
    classic.hudText = SDL_Color{235, 235, 240, 255};
    classic.vignette = SDL_Color{0, 0, 0, 255};
    classic.shieldColor = SDL_Color{235, 235, 240, 255};
    classic.glowIntensity = 0.0f;
    classic.bonusColors = {
        SDL_Color{90, 220, 110, 255},   // Life
        SDL_Color{240, 210, 70, 255},   // Speed
        SDL_Color{170, 110, 230, 255},  // Size
        SDL_Color{250, 190, 60, 255},   // Score
        SDL_Color{80, 210, 220, 255},   // Control
        SDL_Color{100, 150, 240, 255},  // Vision
        SDL_Color{235, 235, 240, 255},  // Shield
    };
    classic.drawbackColors = {
        SDL_Color{220, 60, 60, 255},   // Life
        SDL_Color{170, 130, 30, 255},  // Speed
        SDL_Color{110, 60, 150, 255},  // Size
        SDL_Color{160, 100, 30, 255},  // Score
        SDL_Color{40, 130, 150, 255},  // Control
        SDL_Color{60, 80, 150, 255},   // Vision
        SDL_Color{235, 235, 240, 255}, // Shield
    };
    themes_.push_back(classic);

    auto deriveTheme = [](std::string id, std::string name, BlockStyle style,
                           BackgroundStyle bgStyle, SDL_Color bgTop, SDL_Color bgBottom,
                           SDL_Color gridLine, SDL_Color head, SDL_Color body,
                           SDL_Color hudBg, SDL_Color hudText, SDL_Color vignette,
                           float glow) {
        Theme theme;
        theme.id = std::move(id);
        theme.displayName = std::move(name);
        theme.blockStyle = style;
        theme.backgroundStyle = bgStyle;
        theme.backgroundTop = bgTop;
        theme.backgroundBottom = bgBottom;
        theme.gridLine = gridLine;
        theme.snakeHead = head;
        theme.snakeBody = body;
        theme.hudBarBackground = hudBg;
        theme.hudText = hudText;
        theme.vignette = vignette;
        theme.shieldColor = hudText;
        theme.glowIntensity = glow;
        theme.bonusColors = deriveBonusColors(head);
        theme.drawbackColors = deriveDrawbackColors(head);
        return theme;
    };

    themes_.push_back(deriveTheme(
        "prism", "Prism", BlockStyle::Bevel3D, BackgroundStyle::VerticalGradient,
        SDL_Color{14, 10, 26, 255}, SDL_Color{6, 4, 14, 255}, SDL_Color{70, 58, 100, 45},
        SDL_Color{110, 255, 200, 255}, SDL_Color{35, 150, 140, 255}, SDL_Color{28, 20, 46, 255},
        SDL_Color{235, 235, 250, 255}, SDL_Color{5, 0, 15, 255}, 1.0f));

    themes_.push_back(deriveTheme(
        "neon", "Neon Arcade", BlockStyle::Bevel3D, BackgroundStyle::Solid,
        SDL_Color{8, 8, 14, 255}, SDL_Color{8, 8, 14, 255}, SDL_Color{45, 22, 65, 90},
        SDL_Color{255, 60, 220, 255}, SDL_Color{170, 30, 150, 255}, SDL_Color{24, 10, 34, 255},
        SDL_Color{255, 230, 255, 255}, SDL_Color{4, 0, 8, 255}, 1.2f));

    themes_.push_back(deriveTheme(
        "sunset", "Sunset Dunes", BlockStyle::Bevel3D, BackgroundStyle::VerticalGradient,
        SDL_Color{80, 40, 60, 255}, SDL_Color{28, 14, 38, 255}, SDL_Color{130, 80, 70, 55},
        SDL_Color{255, 170, 80, 255}, SDL_Color{210, 100, 55, 255}, SDL_Color{55, 28, 38, 255},
        SDL_Color{255, 235, 220, 255}, SDL_Color{20, 8, 16, 255}, 0.8f));

    themes_.push_back(deriveTheme(
        "frost", "Frostbyte", BlockStyle::Bevel3D, BackgroundStyle::VerticalGradient,
        SDL_Color{18, 34, 55, 255}, SDL_Color{7, 13, 25, 255}, SDL_Color{85, 125, 165, 55},
        SDL_Color{160, 230, 255, 255}, SDL_Color{70, 145, 195, 255}, SDL_Color{18, 32, 48, 255},
        SDL_Color{220, 240, 255, 255}, SDL_Color{3, 8, 16, 255}, 0.9f));

    themes_.push_back(deriveTheme(
        "terminal", "Terminal Green", BlockStyle::Flat, BackgroundStyle::Solid,
        SDL_Color{5, 10, 5, 255}, SDL_Color{5, 10, 5, 255}, SDL_Color{16, 36, 16, 255},
        SDL_Color{80, 255, 90, 255}, SDL_Color{35, 175, 55, 255}, SDL_Color{10, 20, 10, 255},
        SDL_Color{80, 255, 90, 255}, SDL_Color{0, 0, 0, 255}, 0.0f));

    themes_.push_back(deriveTheme(
        "bloodmoon", "Blood Moon", BlockStyle::Bevel3D, BackgroundStyle::VerticalGradient,
        SDL_Color{42, 8, 10, 255}, SDL_Color{10, 2, 4, 255}, SDL_Color{95, 22, 22, 55},
        SDL_Color{255, 80, 70, 255}, SDL_Color{175, 30, 30, 255}, SDL_Color{36, 10, 12, 255},
        SDL_Color{255, 220, 215, 255}, SDL_Color{10, 0, 0, 255}, 1.1f));

    themes_.push_back(deriveTheme(
        "pastel", "Pastel Dream", BlockStyle::Bevel3D, BackgroundStyle::VerticalGradient,
        SDL_Color{250, 235, 245, 255}, SDL_Color{228, 238, 250, 255}, SDL_Color{205, 195, 220, 130},
        SDL_Color{255, 150, 190, 255}, SDL_Color{205, 115, 165, 255}, SDL_Color{246, 236, 246, 255},
        SDL_Color{90, 70, 100, 255}, SDL_Color{200, 190, 210, 255}, 0.5f));
}

const Theme& ThemeCatalog::themeAt(size_t index) const {
    static const Theme kFallback{};
    if (themes_.empty()) {
        return kFallback;
    }
    if (index >= themes_.size()) {
        index = themes_.size() - 1;
    }
    return themes_[index];
}

size_t ThemeCatalog::indexForId(const std::string& id) const {
    for (size_t i = 0; i < themes_.size(); ++i) {
        if (themes_[i].id == id) {
            return i;
        }
    }
    return 0;
}

}  // namespace snake_engine

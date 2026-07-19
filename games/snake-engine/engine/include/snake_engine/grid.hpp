#pragma once

#include "snake_engine/math.hpp"

namespace snake_engine {

// Describes the play field: how many cells wide/tall it is, and how many
// screen pixels one cell occupies. Kept separate from rendering so the
// editor can later resize/reshape the field without touching gameplay code.
struct Grid {
    int width;
    int height;
    int cellSizePixels;

    [[nodiscard]] bool inBounds(Vec2i cell) const {
        return cell.x >= 0 && cell.y >= 0 && cell.x < width && cell.y < height;
    }

    [[nodiscard]] Vec2f cellCenterPixels(Vec2i cell) const {
        float half = static_cast<float>(cellSizePixels) * 0.5f;
        return {static_cast<float>(cell.x * cellSizePixels) + half,
                static_cast<float>(cell.y * cellSizePixels) + half};
    }

    [[nodiscard]] Vec2f pixelsToGridSpace(Vec2f pixels) const {
        return {pixels.x / static_cast<float>(cellSizePixels),
                pixels.y / static_cast<float>(cellSizePixels)};
    }

    [[nodiscard]] int widthPixels() const { return width * cellSizePixels; }
    [[nodiscard]] int heightPixels() const { return height * cellSizePixels; }
};

}  // namespace snake_engine

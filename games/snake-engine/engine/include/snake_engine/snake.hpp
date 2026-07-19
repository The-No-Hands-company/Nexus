#pragma once

#include <deque>

#include "snake_engine/math.hpp"

namespace snake_engine {

// A grid-locked snake that is steered by the player's cursor rather than
// discrete key presses: every tick it picks whichever of the four cardinal
// directions best matches the vector from its head to the cursor, subject to
// "you can't reverse into your own neck".
class Snake {
public:
    Snake(Vec2i startPosition, int startLength);

    void reset(Vec2i startPosition, int startLength);

    // `cursorGridPos` is the cursor position expressed in the same grid
    // space as the snake body (fractional, since the cursor moves smoothly).
    void steerToward(Vec2f cursorGridPos);

    // Advances the snake one grid cell in its current direction. Returns
    // false if this move caused the snake to eat its own body.
    bool tick();

    void grow(int segments) { pendingGrowth_ += segments; }

    // Removes segments from the tail. Returns false if the snake shrank to
    // nothing (a "life ending" Size drawback can do this).
    bool shrink(int segments);

    [[nodiscard]] const std::deque<Vec2i>& body() const { return body_; }
    [[nodiscard]] const Vec2i& head() const { return body_.front(); }
    [[nodiscard]] Vec2i direction() const { return direction_; }
    [[nodiscard]] size_t length() const { return body_.size(); }

private:
    std::deque<Vec2i> body_;
    Vec2i direction_{1, 0};
    int pendingGrowth_ = 0;

    [[nodiscard]] bool wouldReverse(Vec2i candidate) const;
};

}  // namespace snake_engine

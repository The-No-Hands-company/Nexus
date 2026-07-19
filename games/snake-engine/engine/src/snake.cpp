#include "snake_engine/snake.hpp"

#include <algorithm>
#include <cmath>

namespace snake_engine {

Snake::Snake(Vec2i startPosition, int startLength) {
    reset(startPosition, startLength);
}

void Snake::reset(Vec2i startPosition, int startLength) {
    body_.clear();
    direction_ = {1, 0};
    pendingGrowth_ = 0;
    startLength = std::max(startLength, 1);
    for (int i = 0; i < startLength; ++i) {
        body_.push_back(Vec2i{startPosition.x - i, startPosition.y});
    }
}

bool Snake::wouldReverse(Vec2i candidate) const {
    if (body_.size() < 2) {
        return false;
    }
    return candidate == Vec2i{-direction_.x, -direction_.y};
}

void Snake::steerToward(Vec2f cursorGridPos) {
    Vec2f headPos = toVec2f(head());
    Vec2f delta = cursorGridPos - headPos;

    if (std::abs(delta.x) < 0.05f && std::abs(delta.y) < 0.05f) {
        return;  // cursor is essentially on the head, keep current heading
    }

    Vec2i candidate;
    if (std::abs(delta.x) >= std::abs(delta.y)) {
        candidate = Vec2i{delta.x >= 0 ? 1 : -1, 0};
    } else {
        candidate = Vec2i{0, delta.y >= 0 ? 1 : -1};
    }

    if (wouldReverse(candidate)) {
        // Fall back to the perpendicular axis rather than let the player
        // steer directly backward into their own neck.
        candidate = std::abs(delta.x) >= std::abs(delta.y) ? Vec2i{0, delta.y >= 0 ? 1 : -1}
                                                             : Vec2i{delta.x >= 0 ? 1 : -1, 0};
        if (wouldReverse(candidate)) {
            return;
        }
    }

    direction_ = candidate;
}

bool Snake::tick() {
    Vec2i newHead = head() + direction_;
    bool selfCollision = std::any_of(body_.begin(), body_.end(),
                                      [&](const Vec2i& segment) { return segment == newHead; });

    body_.push_front(newHead);
    if (pendingGrowth_ > 0) {
        --pendingGrowth_;
    } else {
        body_.pop_back();
    }

    return !selfCollision;
}

bool Snake::shrink(int segments) {
    segments = std::max(segments, 0);
    while (segments > 0 && body_.size() > 1) {
        body_.pop_back();
        --segments;
    }
    if (segments > 0) {
        // Would need to remove more than we have left: the snake dies.
        body_.clear();
        return false;
    }
    return true;
}

}  // namespace snake_engine

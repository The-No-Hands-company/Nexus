#pragma once

#include <cmath>
#include <cstdint>

namespace snake_engine {

struct Vec2i {
    int x = 0;
    int y = 0;

    friend constexpr bool operator==(const Vec2i& a, const Vec2i& b) {
        return a.x == b.x && a.y == b.y;
    }
    friend constexpr bool operator!=(const Vec2i& a, const Vec2i& b) { return !(a == b); }
    friend constexpr Vec2i operator+(const Vec2i& a, const Vec2i& b) {
        return {a.x + b.x, a.y + b.y};
    }
    friend constexpr Vec2i operator-(const Vec2i& a, const Vec2i& b) {
        return {a.x - b.x, a.y - b.y};
    }
};

struct Vec2f {
    float x = 0.0f;
    float y = 0.0f;

    friend constexpr Vec2f operator+(const Vec2f& a, const Vec2f& b) {
        return {a.x + b.x, a.y + b.y};
    }
    friend constexpr Vec2f operator-(const Vec2f& a, const Vec2f& b) {
        return {a.x - b.x, a.y - b.y};
    }
    friend constexpr Vec2f operator*(const Vec2f& a, float s) { return {a.x * s, a.y * s}; }

    [[nodiscard]] float length() const { return std::sqrt(x * x + y * y); }
};

inline Vec2f toVec2f(const Vec2i& v) {
    return {static_cast<float>(v.x), static_cast<float>(v.y)};
}

template <typename T>
T clamp(T value, T lo, T hi) {
    return value < lo ? lo : (value > hi ? hi : value);
}

}  // namespace snake_engine

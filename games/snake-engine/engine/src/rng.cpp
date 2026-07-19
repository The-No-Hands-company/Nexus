#include "snake_engine/rng.hpp"

#include <numeric>

namespace snake_engine {

Rng::Rng() : engine_(std::random_device{}()) {}

Rng::Rng(uint64_t seed) : engine_(seed) {}

float Rng::nextFloat01() {
    std::uniform_real_distribution<float> dist(0.0f, 1.0f);
    return dist(engine_);
}

int Rng::nextInt(int minInclusive, int maxInclusive) {
    std::uniform_int_distribution<int> dist(minInclusive, maxInclusive);
    return dist(engine_);
}

size_t Rng::weightedIndex(const std::vector<double>& weights) {
    double total = std::accumulate(weights.begin(), weights.end(), 0.0);
    if (total <= 0.0) {
        return 0;
    }
    double roll = static_cast<double>(nextFloat01()) * total;
    double cumulative = 0.0;
    for (size_t i = 0; i < weights.size(); ++i) {
        cumulative += weights[i];
        if (roll <= cumulative) {
            return i;
        }
    }
    return weights.size() - 1;
}

}  // namespace snake_engine

#pragma once

#include <cstdint>
#include <random>
#include <vector>

namespace snake_engine {

// Central RNG used for anything that must be "drawn up algorithmically" —
// item categories, bonus/drawback polarity, and severity tiers. Never driven
// by player choice. Deterministic when seeded explicitly (useful for tests
// and for reproducible editor/replay tooling later on).
class Rng {
public:
    Rng();
    explicit Rng(uint64_t seed);

    float nextFloat01();
    int nextInt(int minInclusive, int maxInclusive);

    // Picks an index in [0, weights.size()) with probability proportional to
    // each weight. Weights must be non-negative and sum > 0.
    size_t weightedIndex(const std::vector<double>& weights);

private:
    std::mt19937_64 engine_;
};

}  // namespace snake_engine

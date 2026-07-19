// Minimal, dependency-free test harness: each CHECK that fails prints and
// the process exits non-zero, which is all `ctest` needs. No test framework
// dependency keeps "clone and build" friction-free on every platform.

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <vector>

#include "snake_engine/effect.hpp"
#include "snake_engine/effect_catalog.hpp"
#include "snake_engine/grid.hpp"
#include "snake_engine/rng.hpp"
#include "snake_engine/snake.hpp"
#include "snake_engine/upgrades.hpp"

using namespace snake_engine;

namespace {
int g_failures = 0;

void check(bool condition, const char* description) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", description);
        ++g_failures;
    }
}

bool nearlyEqual(float a, float b, float epsilon = 0.001f) {
    return std::fabs(a - b) <= epsilon;
}
}  // namespace

void testResolveLifeDelta() {
    check(nearlyEqual(resolveLifeDelta(MagnitudeType::FlatLife, -1.0, 50.0f), -1.0f),
          "flat -1 life delta is exactly -1 regardless of current life");

    // The headline rule: a Catastrophic drawback removes 99% of whatever
    // life the player has *right now*, not of their max life.
    float delta = resolveLifeDelta(MagnitudeType::PercentOfCurrentLife, -0.99, 40.0f);
    check(nearlyEqual(delta, -39.6f), "-99% of current life (40) is -39.6");

    float half = resolveLifeDelta(MagnitudeType::PercentOfCurrentLife, -0.5, 200.0f);
    check(nearlyEqual(half, -100.0f), "-50% of current life (200) is -100");
}

void testEffectCatalogDefaults() {
    EffectCatalog catalog;
    catalog.loadBuiltinDefaults();
    check(!catalog.definitions().empty(), "builtin catalog is non-empty");

    bool foundCatastrophic = false;
    for (const auto& def : catalog.definitions()) {
        if (def.category != EffectCategory::Life) continue;
        for (const auto& tier : def.drawbackTiers) {
            if (tier.name == "Catastrophic") {
                foundCatastrophic = true;
                check(nearlyEqual(static_cast<float>(tier.magnitude), -0.99f),
                      "Catastrophic life drawback magnitude is -0.99");
                check(tier.weight < 1.0, "Catastrophic drawback is rare (weight < 1.0)");
            }
        }
    }
    check(foundCatastrophic, "builtin Life effect defines a Catastrophic drawback tier");

    Rng rng(12345);
    RolledEffect rolled = catalog.rollRandomEffect(rng, 0.0f);
    check(!rolled.defId.empty(), "rollRandomEffect returns a populated effect");
}

void testWeightedRngDistribution() {
    Rng rng(42);
    std::vector<double> weights = {1.0, 3.0};  // expect ~25% index 0, ~75% index 1
    int hitsForIndexOne = 0;
    constexpr int kTrials = 20000;
    for (int i = 0; i < kTrials; ++i) {
        if (rng.weightedIndex(weights) == 1) {
            ++hitsForIndexOne;
        }
    }
    double ratio = static_cast<double>(hitsForIndexOne) / kTrials;
    check(ratio > 0.70 && ratio < 0.80, "weightedIndex approximates the given weight ratio");
}

void testSnakeMovementGrowthShrink() {
    Snake snake(Vec2i{10, 10}, 4);
    check(snake.length() == 4, "snake starts at requested length");

    Vec2i headBefore = snake.head();
    snake.steerToward(Vec2f{20.0f, 10.0f});  // straight ahead of default rightward direction
    bool alive = snake.tick();
    check(alive, "moving into empty space keeps the snake alive");
    check(snake.head() != headBefore, "head moved after tick()");
    check(snake.length() == 4, "length unchanged without pending growth");

    // grow() is realized gradually: one extra segment is kept per tick until
    // the pending amount is used up (classic "tail waits" snake growth).
    snake.grow(3);
    snake.tick();
    snake.tick();
    snake.tick();
    check(snake.length() == 7, "grow(3) adds exactly 3 segments over the following 3 ticks");

    bool stillAlive = snake.shrink(2);
    check(stillAlive, "shrinking by less than the length keeps it alive");
    check(snake.length() == 5, "shrink(2) removes exactly 2 segments");

    bool diedFromShrink = snake.shrink(50);
    check(!diedFromShrink, "shrinking past zero segments reports death");
    check(snake.length() == 0, "a lethal shrink empties the body");
}

void testGridConversions() {
    Grid grid{20, 15, 16};
    check(grid.inBounds(Vec2i{0, 0}), "origin cell is in bounds");
    check(grid.inBounds(Vec2i{19, 14}), "bottom-right cell is in bounds");
    check(!grid.inBounds(Vec2i{20, 0}), "one past width is out of bounds");
    check(!grid.inBounds(Vec2i{-1, 0}), "negative coordinate is out of bounds");

    Vec2f gridSpace = grid.pixelsToGridSpace(Vec2f{32.0f, 48.0f});
    check(nearlyEqual(gridSpace.x, 2.0f) && nearlyEqual(gridSpace.y, 3.0f),
          "pixelsToGridSpace divides by cell size");
}

void testUpgradesAndEssence() {
    MetaProgress progress;
    progress.essence = 100;

    int64_t firstCost = costForNextLevel(UpgradeKind::Vitality, 0);
    check(firstCost == 50, "first Vitality level costs the base 50 essence");

    bool bought = progress.purchase(UpgradeKind::Vitality);
    check(bought, "purchase succeeds when essence covers the cost");
    check(progress.upgrades.vitality == 1, "purchase increments the upgrade level");
    check(progress.essence == 100 - firstCost, "purchase deducts exactly the cost");

    progress.essence = 0;
    bool failedBuy = progress.purchase(UpgradeKind::Luck);
    check(!failedBuy, "purchase fails when essence is insufficient");
    check(progress.upgrades.luck == 0, "a failed purchase does not change the level");

    check(MetaProgress::essenceForScore(400) == 100, "essenceForScore uses the documented ratio");
}

int main() {
    testResolveLifeDelta();
    testEffectCatalogDefaults();
    testWeightedRngDistribution();
    testSnakeMovementGrowthShrink();
    testGridConversions();
    testUpgradesAndEssence();

    if (g_failures > 0) {
        std::fprintf(stderr, "\n%d check(s) failed\n", g_failures);
        return 1;
    }
    std::printf("All checks passed.\n");
    return 0;
}

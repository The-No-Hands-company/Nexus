#pragma once

#include <string>

#include "snake_engine/upgrades.hpp"

namespace snake_engine {

// Simple JSON save file for persistent meta-progression. Deliberately
// dependency-free beyond nlohmann::json so it works identically on every
// supported platform without a platform-specific "app data directory" API —
// callers pass whatever path makes sense for their OS (see game/src/main.cpp
// for the default).
bool saveMetaProgress(const std::string& path, const MetaProgress& progress);
bool loadMetaProgress(const std::string& path, MetaProgress& outProgress);

}  // namespace snake_engine

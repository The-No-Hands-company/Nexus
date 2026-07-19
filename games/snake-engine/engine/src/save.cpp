#include "snake_engine/save.hpp"

#include <fstream>
#include <nlohmann/json.hpp>

namespace snake_engine {

using json = nlohmann::json;

bool saveMetaProgress(const std::string& path, const MetaProgress& progress) {
    json root;
    root["essence"] = progress.essence;
    root["upgrades"] = {
        {"vitality", progress.upgrades.vitality},
        {"luck", progress.upgrades.luck},
        {"insurance", progress.upgrades.insurance},
        {"regeneration", progress.upgrades.regeneration},
    };

    std::ofstream file(path);
    if (!file.is_open()) {
        return false;
    }
    file << root.dump(2);
    return true;
}

bool loadMetaProgress(const std::string& path, MetaProgress& outProgress) {
    std::ifstream file(path);
    if (!file.is_open()) {
        return false;
    }

    json root;
    try {
        file >> root;
    } catch (const json::parse_error&) {
        return false;
    }

    outProgress.essence = root.value("essence", int64_t{0});
    if (root.contains("upgrades")) {
        const auto& u = root["upgrades"];
        outProgress.upgrades.vitality = u.value("vitality", 0);
        outProgress.upgrades.luck = u.value("luck", 0);
        outProgress.upgrades.insurance = u.value("insurance", 0);
        outProgress.upgrades.regeneration = u.value("regeneration", 0);
    }
    return true;
}

}  // namespace snake_engine

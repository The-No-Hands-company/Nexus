import { useEffect, useState } from "react";
import { useStore } from "../store";
import { invoke } from "../invoke";
import type { MarketplacePlugin, PluginReview, PluginInstall } from "../store";

/**
 * PluginMarketplace — Browse, search, review, and install plugins.
 */
export default function PluginMarketplace({ serverId }: { serverId?: string }) {
  const {
    marketplacePlugins,
    loadMarketplacePlugins,
    pluginReviews,
    loadPluginReviews,
    serverPluginInstalls,
    loadServerPluginInstalls,
  } = useStore();

  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("");
  const [selectedPlugin, setSelectedPlugin] = useState<MarketplacePlugin | null>(null);
  const [installing, setInstalling] = useState(false);
  const [reviewRating, setReviewRating] = useState(5);
  const [reviewTitle, setReviewTitle] = useState("");
  const [reviewBody, setReviewBody] = useState("");
  const [submittingReview, setSubmittingReview] = useState(false);

  const installs = serverId ? serverPluginInstalls[serverId] ?? [] : [];
  const installedPluginIds = new Set(installs.map((i) => i.pluginId));

  useEffect(() => {
    loadMarketplacePlugins(search || undefined, category || undefined);
  }, [search, category, loadMarketplacePlugins]);

  useEffect(() => {
    if (serverId) loadServerPluginInstalls(serverId);
  }, [serverId, loadServerPluginInstalls]);

  const handleInstall = async (plugin: MarketplacePlugin) => {
    if (!serverId) return;
    setInstalling(true);
    try {
      await invoke<PluginInstall>("install_server_plugin", {
        serverId,
        pluginId: plugin.id,
        version: plugin.version,
      });
      await loadServerPluginInstalls(serverId);
    } catch (e) {
      console.error("install error", e);
    } finally {
      setInstalling(false);
    }
  };

  const handleUninstall = async (pluginId: string) => {
    if (!serverId) return;
    try {
      await invoke("uninstall_server_plugin", { serverId, pluginId });
      await loadServerPluginInstalls(serverId);
    } catch (e) {
      console.error("uninstall error", e);
    }
  };

  const handleToggle = async (pluginId: string, enabled: boolean) => {
    if (!serverId) return;
    try {
      await invoke("toggle_server_plugin", { serverId, pluginId, enabled });
      await loadServerPluginInstalls(serverId);
    } catch (e) {
      console.error("toggle error", e);
    }
  };

  const openDetail = (plugin: MarketplacePlugin) => {
    setSelectedPlugin(plugin);
    loadPluginReviews(plugin.id);
  };

  const handleSubmitReview = async () => {
    if (!selectedPlugin) return;
    setSubmittingReview(true);
    try {
      await invoke<PluginReview>("submit_plugin_review", {
        pluginId: selectedPlugin.id,
        rating: reviewRating,
        title: reviewTitle || null,
        body: reviewBody || null,
      });
      await loadPluginReviews(selectedPlugin.id);
      setReviewTitle("");
      setReviewBody("");
    } catch (e) {
      console.error("submit review error", e);
    } finally {
      setSubmittingReview(false);
    }
  };

  const reviews = selectedPlugin ? pluginReviews[selectedPlugin.id] ?? [] : [];

  // Stars helper
  const Stars = ({ rating }: { rating: number }) => (
    <span className="text-yellow-400 text-xs">
      {"★".repeat(Math.round(rating))}
      {"☆".repeat(5 - Math.round(rating))}
    </span>
  );

  // ── Detail view ──────────────────────────────────────────────────────────

  if (selectedPlugin) {
    const installed = installedPluginIds.has(selectedPlugin.id);
    const install = installs.find((i) => i.pluginId === selectedPlugin.id);
    return (
      <div className="flex flex-col gap-4 p-4 max-w-2xl">
        <button
          onClick={() => setSelectedPlugin(null)}
          className="text-sm text-indigo-400 hover:text-indigo-300 w-fit"
        >
          ← Back to Marketplace
        </button>

        <div className="flex items-start gap-3">
          {selectedPlugin.iconUrl ? (
            <img src={selectedPlugin.iconUrl} alt="" className="w-12 h-12 rounded-lg" />
          ) : (
            <div className="w-12 h-12 rounded-lg bg-gray-700 flex items-center justify-center text-xl">
              🧩
            </div>
          )}
          <div className="flex-1">
            <h2 className="text-lg font-bold text-white">{selectedPlugin.name}</h2>
            <div className="flex items-center gap-2 text-xs text-gray-400">
              <span>v{selectedPlugin.version}</span>
              <span>·</span>
              <Stars rating={selectedPlugin.avgRating} />
              <span>({selectedPlugin.ratingCount})</span>
              <span>·</span>
              <span>{selectedPlugin.downloads.toLocaleString()} downloads</span>
              {selectedPlugin.isVerified && (
                <span className="text-green-400 text-[10px] bg-green-900/30 px-1.5 py-0.5 rounded">
                  Verified
                </span>
              )}
            </div>
          </div>
          <div>
            {installed ? (
              <div className="flex gap-2">
                <button
                  onClick={() => handleToggle(selectedPlugin.id, !install?.isEnabled)}
                  className="px-3 py-1.5 text-xs bg-gray-700 hover:bg-gray-600 text-white rounded"
                >
                  {install?.isEnabled ? "Disable" : "Enable"}
                </button>
                <button
                  onClick={() => handleUninstall(selectedPlugin.id)}
                  className="px-3 py-1.5 text-xs bg-red-700/50 hover:bg-red-600 text-white rounded"
                >
                  Uninstall
                </button>
              </div>
            ) : serverId ? (
              <button
                onClick={() => handleInstall(selectedPlugin)}
                disabled={installing}
                className="px-4 py-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded"
              >
                {installing ? "Installing…" : "Install"}
              </button>
            ) : null}
          </div>
        </div>

        {selectedPlugin.description && (
          <p className="text-sm text-gray-300">{selectedPlugin.description}</p>
        )}

        {selectedPlugin.tags.length > 0 && (
          <div className="flex gap-1 flex-wrap">
            {selectedPlugin.tags.map((tag) => (
              <span key={tag} className="text-[10px] bg-gray-700/60 text-gray-400 px-1.5 py-0.5 rounded">
                {tag}
              </span>
            ))}
          </div>
        )}

        {/* Reviews */}
        <div className="border-t border-gray-700 pt-4 mt-2">
          <h3 className="text-sm font-semibold text-gray-300 mb-3">Reviews</h3>

          {reviews.length === 0 ? (
            <p className="text-xs text-gray-600">No reviews yet.</p>
          ) : (
            <div className="space-y-3 mb-4">
              {reviews.map((r) => (
                <div key={r.id} className="bg-gray-800/50 rounded p-3">
                  <div className="flex items-center gap-2 mb-1">
                    <Stars rating={r.rating} />
                    {r.title && <span className="text-sm font-medium text-white">{r.title}</span>}
                  </div>
                  {r.body && <p className="text-xs text-gray-400">{r.body}</p>}
                </div>
              ))}
            </div>
          )}

          {/* Submit review */}
          <div className="bg-gray-800/40 rounded p-3 space-y-2">
            <p className="text-xs text-gray-400 font-semibold">Leave a Review</p>
            <div className="flex items-center gap-2">
              <span className="text-xs text-gray-500">Rating:</span>
              {[1, 2, 3, 4, 5].map((n) => (
                <button
                  key={n}
                  onClick={() => setReviewRating(n)}
                  className={`text-lg ${n <= reviewRating ? "text-yellow-400" : "text-gray-600"}`}
                >
                  ★
                </button>
              ))}
            </div>
            <input
              type="text"
              value={reviewTitle}
              onChange={(e) => setReviewTitle(e.target.value)}
              placeholder="Title (optional)"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-1 text-sm text-white"
            />
            <textarea
              value={reviewBody}
              onChange={(e) => setReviewBody(e.target.value)}
              rows={2}
              placeholder="Your review (optional)"
              className="w-full bg-gray-800 border border-gray-700 rounded px-2 py-1 text-sm text-white"
            />
            <button
              onClick={handleSubmitReview}
              disabled={submittingReview}
              className="px-3 py-1.5 text-xs bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded"
            >
              {submittingReview ? "Submitting…" : "Submit Review"}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // ── Browse view ──────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col gap-4 p-4 max-w-2xl">
      <h2 className="text-lg font-bold text-white">Plugin Marketplace</h2>

      {/* Search & filters */}
      <div className="flex gap-2">
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search plugins…"
          className="flex-1 bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
        />
        <select
          value={category}
          onChange={(e) => setCategory(e.target.value)}
          className="bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
        >
          <option value="">All Categories</option>
          <option value="moderation">Moderation</option>
          <option value="fun">Fun</option>
          <option value="utility">Utility</option>
          <option value="music">Music</option>
          <option value="integration">Integration</option>
        </select>
      </div>

      {/* Plugin grid */}
      {marketplacePlugins.length === 0 ? (
        <p className="text-sm text-gray-600 text-center py-8">No plugins found</p>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2">
          {marketplacePlugins.map((plugin) => {
            const installed = installedPluginIds.has(plugin.id);
            return (
              <button
                key={plugin.id}
                onClick={() => openDetail(plugin)}
                className="text-left bg-gray-800/50 border border-gray-700 hover:border-gray-600 rounded-lg p-3 transition-colors"
              >
                <div className="flex items-center gap-2 mb-1">
                  {plugin.iconUrl ? (
                    <img src={plugin.iconUrl} alt="" className="w-8 h-8 rounded" />
                  ) : (
                    <div className="w-8 h-8 rounded bg-gray-700 flex items-center justify-center text-sm">
                      🧩
                    </div>
                  )}
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-white truncate">
                      {plugin.name}
                      {plugin.isVerified && <span className="text-green-400 ml-1">✓</span>}
                    </p>
                    <div className="flex items-center gap-1 text-[10px] text-gray-500">
                      <Stars rating={plugin.avgRating} />
                      <span>({plugin.ratingCount})</span>
                    </div>
                  </div>
                  {installed && (
                    <span className="text-[10px] bg-indigo-500/20 text-indigo-300 px-1.5 py-0.5 rounded">
                      Installed
                    </span>
                  )}
                </div>
                {plugin.description && (
                  <p className="text-xs text-gray-500 line-clamp-2">{plugin.description}</p>
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

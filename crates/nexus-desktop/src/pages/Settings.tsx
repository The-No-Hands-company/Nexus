import { useState } from "react";
import { useStore } from "../store";
import ThemeSwitcher from "../themes/ThemeSwitcher";
import type { PluginManifest } from "../plugins/types";

export default function SettingsPage() {
  const { plugins, enabledPlugins, installPlugin, uninstallPlugin, togglePlugin } = useStore();
  const [urlInput, setUrlInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleInstall = async () => {
    const url = urlInput.trim();
    if (!url) return;
    setError(null);
    setLoading(true);
    try {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const manifest: PluginManifest = await res.json();
      if (!manifest.id || !manifest.name || !manifest.url) {
        throw new Error("Invalid plugin manifest (missing id, name, or url).");
      }
      installPlugin(manifest);
      setUrlInput("");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto p-8 text-sm">
      <h1 className="text-2xl font-bold mb-6">Settings</h1>

      {/* ── Appearance ───────────────────────────────── */}
      <section className="mb-10">
        <h2 className="text-base font-semibold mb-4 border-b border-bg-600 pb-2">Appearance</h2>
        <div className="max-w-xs">
          <ThemeSwitcher />
        </div>
      </section>

      {/* ── Plugins ──────────────────────────────────── */}
      <section>
        <h2 className="text-base font-semibold mb-4 border-b border-bg-600 pb-2">Plugins</h2>

        {/* Install form */}
        <div className="mb-6">
          <p className="text-muted mb-2">
            Enter the URL of a plugin manifest JSON to install it.
          </p>
          <div className="flex gap-2 max-w-lg">
            <input
              type="url"
              placeholder="https://example.com/my-plugin/manifest.json"
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleInstall()}
              className="input flex-1"
            />
            <button
              onClick={handleInstall}
              disabled={loading || !urlInput.trim()}
              className="btn-primary whitespace-nowrap disabled:opacity-50"
            >
              {loading ? "Installing…" : "Install"}
            </button>
          </div>
          {error && <p className="mt-2 text-dnd text-xs">{error}</p>}
        </div>

        {/* Installed plugins list */}
        {plugins.length === 0 ? (
          <p className="text-muted">No plugins installed.</p>
        ) : (
          <ul className="flex flex-col gap-3 max-w-2xl">
            {plugins.map((p) => {
              const enabled = enabledPlugins.includes(p.id);
              return (
                <li
                  key={p.id}
                  className="flex items-start gap-4 bg-bg-800 rounded-lg p-4"
                >
                  {p.iconUrl && (
                    <img
                      src={p.iconUrl}
                      alt=""
                      className="w-10 h-10 rounded-lg flex-shrink-0 object-cover"
                    />
                  )}
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">{p.name}</div>
                    <div className="text-xs text-muted">{p.id} · v{p.version}</div>
                    {p.description && (
                      <div className="text-xs text-muted mt-1">{p.description}</div>
                    )}
                  </div>
                  <div className="flex gap-2 flex-shrink-0">
                    <button
                      onClick={() => togglePlugin(p.id)}
                      className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                        enabled
                          ? "bg-accent-600 hover:bg-accent-500 text-white"
                          : "bg-bg-600 hover:bg-bg-500 text-muted hover:text-white"
                      }`}
                    >
                      {enabled ? "Disable" : "Enable"}
                    </button>
                    <button
                      onClick={() => uninstallPlugin(p.id)}
                      className="px-3 py-1 rounded text-xs font-medium bg-bg-600 hover:bg-dnd text-muted hover:text-white transition-colors"
                    >
                      Uninstall
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}

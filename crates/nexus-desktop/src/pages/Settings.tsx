import { useState, useEffect } from "react";
import { useStore } from "../store";
import { invoke } from "../invoke";
import ThemeSwitcher from "../themes/ThemeSwitcher";
import type { PluginManifest } from "../plugins/types";
import { formatDistanceToNow } from "date-fns";

interface E2eeDevice {
  id: string;
  userId: string;
  name: string;
  deviceType: string;
  lastSeenAt?: string;
  verified: boolean;
  createdAt: string;
}

export default function SettingsPage() {
  const { plugins, enabledPlugins, installPlugin, uninstallPlugin, togglePlugin, session, setSession } = useStore();
  const [urlInput, setUrlInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Profile section state
  const [displayName, setDisplayName] = useState(session?.displayName ?? "");
  const [avatarUrl, setAvatarUrl] = useState(session?.avatar ?? "");
  const [profileSaving, setProfileSaving] = useState(false);
  const [profileMsg, setProfileMsg] = useState<string | null>(null);

  // ── Notification preferences (persisted to localStorage) ──────────────────
  const [notifEnabled, setNotifEnabled] = useState(
    () => localStorage.getItem("nexus:notif:enabled") !== "false"
  );
  const [notifMentionsOnly, setNotifMentionsOnly] = useState(
    () => localStorage.getItem("nexus:notif:mentionsOnly") === "true"
  );
  const [notifSound, setNotifSound] = useState(
    () => localStorage.getItem("nexus:notif:sound") !== "false"
  );
  const toggleNotif = (key: string, value: boolean) => {
    localStorage.setItem(`nexus:notif:${key}`, String(value));
    if (key === "enabled") setNotifEnabled(value);
    if (key === "mentionsOnly") setNotifMentionsOnly(value);
    if (key === "sound") setNotifSound(value);
  };

  // ── Privacy preferences ────────────────────────────────────────────────────
  const [privacyReadReceipts, setPrivacyReadReceipts] = useState(
    () => localStorage.getItem("nexus:privacy:readReceipts") !== "false"
  );
  const [privacyOnlineStatus, setPrivacyOnlineStatus] = useState(
    () => localStorage.getItem("nexus:privacy:onlineStatus") !== "false"
  );
  const togglePrivacy = (key: string, value: boolean) => {
    localStorage.setItem(`nexus:privacy:${key}`, String(value));
    if (key === "readReceipts") setPrivacyReadReceipts(value);
    if (key === "onlineStatus") setPrivacyOnlineStatus(value);
  };

  // ── E2EE Devices ───────────────────────────────────────────────────────────
  const [devices, setDevices] = useState<E2eeDevice[]>([]);
  const [devicesLoading, setDevicesLoading] = useState(false);
  const [devicesError, setDevicesError] = useState<string | null>(null);
  useEffect(() => {
    if (!session) return;
    setDevicesLoading(true);
    invoke<E2eeDevice[]>("list_devices", {})
      .then(setDevices)
      .catch((e) => setDevicesError(String(e)))
      .finally(() => setDevicesLoading(false));
  }, [session]);

  const saveProfile = async () => {
    if (!session) return;
    setProfileSaving(true);
    setProfileMsg(null);
    try {
      await invoke("update_profile", {
        displayName: displayName.trim() || null,
        avatarUrl: avatarUrl.trim() || null,
      });
      setSession({
        ...session,
        displayName: displayName.trim() || undefined,
        avatar: avatarUrl.trim() || undefined,
      });
      setProfileMsg("Saved!");
      setTimeout(() => setProfileMsg(null), 2500);
    } catch (e) {
      setProfileMsg(`Error: ${String(e)}`);
    } finally {
      setProfileSaving(false);
    }
  };

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

      {/* ── Profile ──────────────────────────────────────────── */}
      <section className="mb-10">
        <h2 className="text-base font-semibold mb-4 border-b border-bg-600 pb-2">Profile</h2>
        <div className="flex gap-6 items-start max-w-lg">
          {/* Avatar preview */}
          <div className="shrink-0">
            {avatarUrl ? (
              <img
                src={avatarUrl}
                alt="avatar"
                className="w-16 h-16 rounded-full object-cover"
                onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
              />
            ) : (
              <div className="w-16 h-16 rounded-full bg-accent-500 flex items-center justify-center text-2xl font-bold text-white select-none">
                {(session?.displayName ?? session?.username ?? '?')[0].toUpperCase()}
              </div>
            )}
          </div>

          <div className="flex flex-col gap-3 flex-1">
            <div>
              <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">Display Name</label>
              <input
                className="input w-full"
                placeholder={session?.username ?? ''}
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
              />
            </div>
            <div>
              <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">Avatar URL</label>
              <input
                className="input w-full"
                placeholder="https://example.com/avatar.png"
                value={avatarUrl}
                onChange={(e) => setAvatarUrl(e.target.value)}
              />
            </div>
            <div className="flex items-center gap-3">
              <button
                onClick={saveProfile}
                disabled={profileSaving}
                className="btn-primary disabled:opacity-50"
              >
                {profileSaving ? 'Saving…' : 'Save Changes'}
              </button>
              {profileMsg && (
                <span className={profileMsg.startsWith('Error') ? 'text-dnd text-xs' : 'text-green-400 text-xs'}>
                  {profileMsg}
                </span>
              )}
            </div>
          </div>
        </div>
      </section>
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

      {/* ── Notifications ──────────────────────────────── */}
      <section className="mb-10">
        <h2 className="text-base font-semibold mb-4 border-b border-bg-600 pb-2">Notifications</h2>
        <div className="flex flex-col gap-4 max-w-md">
          <ToggleRow
            label="Enable notifications"
            description="Show desktop notifications for new messages"
            value={notifEnabled}
            onChange={(v) => toggleNotif("enabled", v)}
          />
          <ToggleRow
            label="Mentions only"
            description="Only notify for @mentions and direct messages"
            value={notifMentionsOnly}
            onChange={(v) => toggleNotif("mentionsOnly", v)}
          />
          <ToggleRow
            label="Notification sounds"
            description="Play a sound when a notification arrives"
            value={notifSound}
            onChange={(v) => toggleNotif("sound", v)}
          />
        </div>
      </section>

      {/* ── Privacy ─────────────────────────────────────── */}
      <section className="mb-10">
        <h2 className="text-base font-semibold mb-4 border-b border-bg-600 pb-2">Privacy</h2>
        <div className="flex flex-col gap-4 max-w-md">
          <ToggleRow
            label="Show online status"
            description="Let others see when you are online"
            value={privacyOnlineStatus}
            onChange={(v) => togglePrivacy("onlineStatus", v)}
          />
          <ToggleRow
            label="Read receipts"
            description="Show others when you have read their messages"
            value={privacyReadReceipts}
            onChange={(v) => togglePrivacy("readReceipts", v)}
          />
        </div>
      </section>

      {/* ── Devices ─────────────────────────────────────── */}
      <section className="mb-10">
        <h2 className="text-base font-semibold mb-4 border-b border-bg-600 pb-2">Devices</h2>
        {devicesError ? (
          <p className="text-sm text-red-400">Failed to load devices: {devicesError}</p>
        ) : devicesLoading ? (
          <p className="text-sm text-muted">Loading devices…</p>
        ) : devices.length === 0 ? (
          <p className="text-sm text-muted">No registered E2EE devices found.</p>
        ) : (
          <ul className="flex flex-col gap-2 max-w-lg">
            {devices.map((d) => (
              <li key={d.id} className="flex items-center gap-3 rounded-lg bg-bg-800 p-3">
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-fg">{d.name}</p>
                  <p className="text-xs text-muted">
                    {d.deviceType} · {d.verified ? "✓ Verified" : "Unverified"}
                  </p>
                  {d.lastSeenAt && (
                    <p className="text-xs text-muted/60">
                      Last seen {formatDistanceToNow(new Date(d.lastSeenAt), { addSuffix: true })}
                    </p>
                  )}
                </div>
                <code className="text-[10px] text-muted/60 font-mono shrink-0">
                  {d.id.slice(0, 8)}…
                </code>
              </li>
            ))}
          </ul>
        )}
      </section>

      {/* ── Connection ──────────────────────────────────── */}
      <section className="mb-10">
        <h2 className="text-base font-semibold mb-4 border-b border-bg-600 pb-2">Connection</h2>
        <div className="max-w-md">
          <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">
            Server URL
          </label>
          <div className="flex items-center gap-2 rounded-lg bg-bg-800 border border-bg-600/50 px-3 py-2">
            <code className="text-sm text-fg flex-1 truncate">
              {session?.serverUrl ?? "Not connected"}
            </code>
            <span className="text-xs text-green-400">●</span>
          </div>
          <p className="text-xs text-muted mt-1">
            To change the server, log out and connect to a different instance.
          </p>
        </div>
      </section>
    </div>
  );
}

// ── ToggleRow helper ─────────────────────────────────────────────────────────
function ToggleRow({
  label,
  description,
  value,
  onChange,
}: {
  label: string;
  description?: string;
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="flex-1">
        <p className="text-sm font-medium text-fg">{label}</p>
        {description && <p className="text-xs text-muted mt-0.5">{description}</p>}
      </div>
      <button
        type="button"
        onClick={() => onChange(!value)}
        aria-pressed={value}
        className={`relative shrink-0 h-5 w-9 rounded-full transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent-500 ${
          value ? "bg-accent-600" : "bg-bg-600/60"
        }`}
      >
        <span
          className={`absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform ${
            value ? "translate-x-4" : "translate-x-0"
          }`}
        />
      </button>
    </div>
  );
}

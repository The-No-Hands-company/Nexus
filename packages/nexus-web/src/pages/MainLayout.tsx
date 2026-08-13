import { useEffect, useRef, useState, lazy, Suspense } from "react";
import { Routes, Route } from "react-router-dom";
import { useStore } from "../store";
import { useGateway } from "../gateway";
import { usePushNotifications } from "../hooks/usePushNotifications";
import ServerList from "../components/ServerList";
import { useVoice } from "../hooks/useVoice";
import ChannelList from "../components/ChannelList";
import ChatView from "../components/ChatView";
import { isEmbedded, shouldShowNotifBanner } from "../embed";

const SettingsPanel   = lazy(() => import("../components/SettingsPanel"));
const JoinServerModal = lazy(() => import("../components/JoinServerModal"));

export default function MainLayout() {
  const { session, loadServers, setSession } = useStore();
  const [showSettings, setShowSettings] = useState(false);
  const [notifDismissed, setNotifDismissed] = useState(
    () => localStorage.getItem("nx_notif_dismissed") === "1"
  );
  const showNotifBanner = shouldShowNotifBanner({
    embedded: isEmbedded(),
    dismissed: notifDismissed,
    supported: typeof Notification !== "undefined",
    hasSession: !!session,
    permissionDefault:
      typeof Notification !== "undefined" && Notification.permission === "default",
  });
  const voice = useVoice();
  const [showJoin, setShowJoin]         = useState(false);

  // Real-time WebSocket gateway
  useGateway();

  // Push notification auto-subscribe
  const { enablePush } = usePushNotifications();
  useEffect(() => {
    if (!session) return;
    if (typeof Notification !== "undefined" && Notification.permission === "granted") {
      enablePush().catch(() => {});
    }
  }, [session, enablePush]);

  // Refresh access token every 10 minutes (TTL = 15 min)
  const refreshRef = useRef<ReturnType<typeof setInterval> | null>(null);
  useEffect(() => {
    const refresh = async () => {
      const { session: s } = useStore.getState();
      if (!s?.refreshToken) return;
      try {
        const res = await fetch(`${s.serverUrl}/api/v1/auth/refresh`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ refresh_token: s.refreshToken }),
        });
        if (!res.ok) return;
        const body = await res.json();
        if (typeof body.access_token === "string") {
          useStore.getState().setSession({
            ...s,
            accessToken: body.access_token,
            refreshToken: body.refresh_token ?? s.refreshToken,
          });
        }
      } catch { /* ignore */ }
    };
    refreshRef.current = setInterval(refresh, 10 * 60 * 1000);
    return () => { if (refreshRef.current) clearInterval(refreshRef.current); };
  }, [session?.refreshToken]);

  useEffect(() => { loadServers(); }, [loadServers]);

  // Keyboard shortcut: Ctrl+, opens Settings
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setShowSettings(true);
      }
      if (e.key === "Escape") {
        setShowSettings(false);
        setShowJoin(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <div className="flex h-screen overflow-hidden bg-bg-900 text-fg">
      {/* Push notification permission banner — bottom-right toast */}
      {showNotifBanner && (
        <div style={{
          position: "fixed", bottom: 20, right: 20, zIndex: 400,
          width: 312,
          background: "var(--bg-700)",
          border: "1px solid rgba(255,255,255,0.09)",
          borderRadius: 14,
          boxShadow: "0 8px 28px rgba(0,0,0,0.45)",
          padding: "14px 16px",
          display: "flex", flexDirection: "column", gap: 10,
        }}>
          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <span style={{ fontSize: 20, lineHeight: 1 }}>🔔</span>
            <div>
              <p style={{ fontSize: 13, fontWeight: 700, color: "var(--fg)", margin: 0 }}>
                Enable notifications
              </p>
              <p style={{ fontSize: 12, color: "var(--muted)", margin: "4px 0 0", lineHeight: 1.5 }}>
                Get alerted for mentions and DMs even when the tab is in the background.
              </p>
            </div>
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <button
              className="nx-btn nx-btn-primary"
              style={{ flex: 1, fontSize: 12, padding: "7px 12px" }}
              onClick={async () => {
                try { await enablePush(); } catch { /* ignore */ }
                localStorage.setItem("nx_notif_dismissed", "1");
                setNotifDismissed(true);
              }}
            >
              Enable
            </button>
            <button
              className="nx-btn nx-btn-ghost"
              style={{ fontSize: 12, padding: "7px 12px" }}
              onClick={() => {
                localStorage.setItem("nx_notif_dismissed", "1");
                setNotifDismissed(true);
              }}
            >
              Not now
            </button>
          </div>
        </div>
      )}

      {/* Column 1 — server rail (passes callbacks down) */}
      <ServerList
        onOpenSettings={() => setShowSettings(true)}
        onOpenJoin={() => setShowJoin(true)}
      />

      {/* Column 2 — channel list */}
      <ChannelList voice={voice} />

      {/* Column 3 — main chat */}
      <div className="flex flex-col flex-1 min-w-0">
        <Routes>
          <Route path="/" element={<EmptyState />} />
          <Route path="/channel/:channelId" element={<ChatView />} />
          <Route path="*" element={<EmptyState />} />
        </Routes>
      </div>

      {/* Modals */}
      <Suspense fallback={null}>
        {showSettings && <SettingsPanel onClose={() => setShowSettings(false)} />}
        {showJoin     && <JoinServerModal onClose={() => setShowJoin(false)} />}
      </Suspense>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex-1 flex flex-col items-center justify-center gap-2 select-none">
      <span className="text-4xl opacity-20">💬</span>
      <p className="text-sm" style={{ color: "var(--muted)" }}>
        Select a channel to start chatting
      </p>
      <p className="text-xs" style={{ color: "var(--muted)", opacity: 0.6 }}>
        Ctrl+, to open settings
      </p>
    </div>
  );
}

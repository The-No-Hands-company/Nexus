import { useEffect, useRef } from "react";
import { Routes, Route } from "react-router-dom";
import { useStore } from "../store";
import { useGateway } from "../gateway";
import { usePushNotifications } from "../hooks/usePushNotifications";
import ServerList from "../components/ServerList";
import ChannelList from "../components/ChannelList";
import ChatView from "../components/ChatView";

export default function MainLayout() {
  const { session, loadServers } = useStore();

  // Real-time WebSocket gateway
  useGateway();

  // Register service worker + auto-resubscribe if permission already granted
  const { enablePush } = usePushNotifications();
  useEffect(() => {
    if (!session) return;
    if (typeof Notification !== "undefined" && Notification.permission === "granted") {
      enablePush().catch(() => {});
    }
  }, [session, enablePush]);

  // Refresh the access token every 10 minutes (TTL is 15 min)
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
      } catch {
        /* ignore — will retry next interval */
      }
    };
    refreshRef.current = setInterval(refresh, 10 * 60 * 1000);
    return () => {
      if (refreshRef.current) clearInterval(refreshRef.current);
    };
  }, [session?.refreshToken]);

  // Load servers on mount
  useEffect(() => {
    loadServers();
  }, [loadServers]);

  return (
    <div className="flex h-screen overflow-hidden bg-bg-900 text-fg">
      {/* Column 1 — Server icon rail */}
      <ServerList />

      {/* Column 2 — Channel list */}
      <ChannelList />

      {/* Column 3 — Main content */}
      <div className="flex flex-col flex-1 min-w-0">
        <Routes>
          <Route
            path="/"
            element={
              <div className="flex-1 flex items-center justify-center text-muted text-sm select-none">
                Select a channel
              </div>
            }
          />
          <Route path="/channel/:channelId" element={<ChatView />} />
          <Route
            path="*"
            element={
              <div className="flex-1 flex items-center justify-center text-muted text-sm select-none">
                Select a channel
              </div>
            }
          />
        </Routes>
      </div>
    </div>
  );
}

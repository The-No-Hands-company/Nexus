import { useEffect, useRef } from "react";
import { Routes, Route } from "react-router-dom";
import { useStore } from "../store";
import { useGateway } from "../hooks/useGateway";
import { usePtt } from "../hooks/usePtt";
import { invoke } from "../invoke";
import ServerList from "../components/ServerList";
import ChannelList from "../components/ChannelList";
import ChatView from "../components/ChatView";
import VoiceChannel from "../components/VoiceChannel";
import SettingsPage from "./Settings";
import FriendsPanel from "../components/FriendsPanel";

export default function MainLayout() {
  const { loadServers, activeServerId, loadChannels } = useStore();

  // Open gateway WebSocket
  useGateway();
  // Listen for PTT events from Tauri
  usePtt();

  // Proactively refresh the access token every 10 minutes (TTL is 15 min).
  // In Tauri mode this updates Rust AppState; in browser mode it updates _token.
  const refreshTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  useEffect(() => {
    const REFRESH_INTERVAL_MS = 10 * 60 * 1000; // 10 minutes
    refreshTimerRef.current = setInterval(() => {
      invoke("refresh_token").catch(() => {
        // If refresh fails the user's session is dead; they'll get 401 and
        // the next action will surface a proper error.
      });
    }, REFRESH_INTERVAL_MS);
    return () => {
      if (refreshTimerRef.current) clearInterval(refreshTimerRef.current);
    };
  }, []);

  useEffect(() => {
    loadServers();
  }, [loadServers]);

  useEffect(() => {
    if (activeServerId) {
      loadChannels(activeServerId);
    }
  }, [activeServerId, loadChannels]);

  return (
    <div className="flex h-full overflow-hidden">
      {/* Column 1: Server list (icon rail) */}
      <ServerList />

      {/* Column 2: Channel list */}
      <ChannelList />

      {/* Column 3: Main content */}
      <div className="flex flex-col flex-1 overflow-hidden">
        <Routes>
          <Route path="/" element={<div className="flex-1 flex items-center justify-center text-muted text-sm">Select a channel</div>} />
          <Route path="/home" element={<FriendsPanel />} />
          <Route path="/channel/:channelId" element={<ChatView />} />
          <Route path="/voice/:channelId" element={<VoiceChannel />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </div>
    </div>
  );
}

import { useEffect } from "react";
import { Routes, Route } from "react-router-dom";
import { useStore } from "../store";
import { useGateway } from "../hooks/useGateway";
import { usePtt } from "../hooks/usePtt";
import ServerList from "../components/ServerList";
import ChannelList from "../components/ChannelList";
import ChatView from "../components/ChatView";
import VoiceChannel from "../components/VoiceChannel";

export default function MainLayout() {
  const { loadServers, activeServerId, loadChannels } = useStore();

  // Open gateway WebSocket
  useGateway();
  // Listen for PTT events from Tauri
  usePtt();

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
          <Route path="/channel/:channelId" element={<ChatView />} />
          <Route path="/voice/:channelId" element={<VoiceChannel />} />
        </Routes>
      </div>
    </div>
  );
}

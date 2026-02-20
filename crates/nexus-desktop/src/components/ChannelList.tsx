import { useState, KeyboardEvent } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useStore } from "../store";
import clsx from "clsx";

export default function ChannelList() {
  const { channels, activeChannelId, setActiveChannel, activeServerId, servers, createChannel, unreadChannels } =
    useStore();
  const navigate = useNavigate();
  const { channelId } = useParams();

  const [creatingType, setCreatingType] = useState<"text" | "voice" | null>(null);
  const [newChannelName, setNewChannelName] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);

  const activeServer = servers.find((s) => s.id === activeServerId);

  const textChannels = channels.filter((c) => c.kind === "text" || c.kind === "announcement");
  const voiceChannels = channels.filter((c) => c.kind === "voice");

  const handleTextChannel = (id: string) => {
    setActiveChannel(id);
    navigate(`/channel/${id}`);
  };

  const handleVoiceChannel = (id: string) => {
    setActiveChannel(id);
    navigate(`/voice/${id}`);
  };

  const startCreate = (type: "text" | "voice") => {
    setCreatingType(type);
    setNewChannelName("");
    setCreateError(null);
  };

  const cancelCreate = () => {
    setCreatingType(null);
    setNewChannelName("");
    setCreateError(null);
  };

  const confirmCreate = async () => {
    if (!newChannelName.trim() || !activeServerId || !creatingType) return;
    try {
      setCreateError(null);
      const ch = await createChannel(activeServerId, newChannelName.trim(), creatingType);
      setCreatingType(null);
      setNewChannelName("");
      if (creatingType === "text") {
        setActiveChannel(ch.id);
        navigate(`/channel/${ch.id}`);
      }
    } catch (e) {
      setCreateError(String(e));
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") confirmCreate();
    if (e.key === "Escape") cancelCreate();
  };

  if (!activeServerId) {
    return (
      <div className="w-56 bg-bg-800 flex flex-col shrink-0 p-3 border-r border-bg-600/40">
        <p className="text-muted text-sm">Select a space</p>
      </div>
    );
  }

  return (
    <div className="w-56 bg-bg-800 flex flex-col shrink-0 overflow-hidden border-r border-bg-600/40">
      {/* Space name header */}
      <div className="px-3 py-3 font-semibold text-sm text-fg no-select shrink-0 flex items-center gap-2">
        <div className="w-5 h-5 rounded bg-accent-500/20 flex items-center justify-center shrink-0">
          <span className="text-accent-400 text-[10px] font-bold leading-none">
            {activeServer?.name.slice(0, 1).toUpperCase()}
          </span>
        </div>
        <span className="truncate">{activeServer?.name ?? "Space"}</span>
      </div>

      <div className="h-px bg-bg-600/40 mx-3 shrink-0" />

      {/* Channel list */}
      <div className="flex-1 overflow-y-auto px-2 py-2 flex flex-col gap-px">

        {/* Error banner */}
        {createError && (
          <div className="text-xs text-red-400 bg-red-950/40 border border-red-800/50 rounded px-2 py-1 mb-1 flex items-center justify-between gap-1">
            <span className="truncate">{createError}</span>
            <button onClick={() => setCreateError(null)} className="shrink-0 hover:text-red-300">✕</button>
          </div>
        )}

        {/* Text channels section */}
        <>
          <div className="flex items-center px-2 py-1 group">
            <p className="flex-1 text-[10px] text-muted/60 uppercase tracking-widest font-medium select-none">
              Rooms
            </p>
            <button
              onClick={() => creatingType === "text" ? cancelCreate() : startCreate("text")}
              className="opacity-0 group-hover:opacity-100 text-muted/60 hover:text-fg transition-all p-0.5 rounded"
              title="Create text room"
            >
              <PlusIcon />
            </button>
          </div>

          {textChannels.map((ch) => (
            <button
              key={ch.id}
              onClick={() => handleTextChannel(ch.id)}
              className={clsx(
                "channel-item w-full text-left",
                (activeChannelId === ch.id || channelId === ch.id) && "active"
              )}
            >
              <TextChannelIcon />
              <span className="truncate text-sm flex-1">{ch.name}</span>
              {ch.isE2ee && (
                <span className="text-[10px] text-green-400/80" title="End-to-end encrypted">
                  E2E
                </span>
              )}
              {unreadChannels[ch.id] && (
                <span className="w-2 h-2 rounded-full bg-white shrink-0" />
              )}
            </button>
          ))}

          {creatingType === "text" && (
            <div className="px-1 py-1">
              <input
                autoFocus
                type="text"
                value={newChannelName}
                onChange={(e) => setNewChannelName(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="room-name"
                maxLength={100}
                className="w-full bg-bg-700 text-sm text-fg rounded px-2 py-1 outline-none focus:ring-1 focus:ring-accent-500 placeholder-muted/50"
              />
              <div className="flex gap-1 mt-1">
                <button onClick={confirmCreate} disabled={!newChannelName.trim()} className="flex-1 text-[11px] bg-accent-500 hover:bg-accent-400 text-white rounded px-2 py-0.5 disabled:opacity-40 transition-colors">Create</button>
                <button onClick={cancelCreate} className="flex-1 text-[11px] bg-bg-600 hover:bg-bg-500 text-muted rounded px-2 py-0.5 transition-colors">Cancel</button>
              </div>
            </div>
          )}
        </>

        {/* Voice channels section */}
        <>
          <div className="h-px bg-bg-600/40 mx-2 my-1.5" />
          <div className="flex items-center px-2 py-1 group">
            <p className="flex-1 text-[10px] text-muted/60 uppercase tracking-widest font-medium select-none">
              Voice
            </p>
            <button
              onClick={() => creatingType === "voice" ? cancelCreate() : startCreate("voice")}
              className="opacity-0 group-hover:opacity-100 text-muted/60 hover:text-fg transition-all p-0.5 rounded"
              title="Create voice channel"
            >
              <PlusIcon />
            </button>
          </div>

          {voiceChannels.map((ch) => (
            <button
              key={ch.id}
              onClick={() => handleVoiceChannel(ch.id)}
              className={clsx(
                "channel-item w-full text-left",
                (activeChannelId === ch.id || channelId === ch.id) && "active"
              )}
            >
              <VoiceChannelIcon />
              <span className="truncate text-sm">{ch.name}</span>
            </button>
          ))}

          {creatingType === "voice" && (
            <div className="px-1 py-1">
              <input
                autoFocus
                type="text"
                value={newChannelName}
                onChange={(e) => setNewChannelName(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="voice-channel"
                maxLength={100}
                className="w-full bg-bg-700 text-sm text-fg rounded px-2 py-1 outline-none focus:ring-1 focus:ring-accent-500 placeholder-muted/50"
              />
              <div className="flex gap-1 mt-1">
                <button onClick={confirmCreate} disabled={!newChannelName.trim()} className="flex-1 text-[11px] bg-accent-500 hover:bg-accent-400 text-white rounded px-2 py-0.5 disabled:opacity-40 transition-colors">Create</button>
                <button onClick={cancelCreate} className="flex-1 text-[11px] bg-bg-600 hover:bg-bg-500 text-muted rounded px-2 py-0.5 transition-colors">Cancel</button>
              </div>
            </div>
          )}
        </>
      </div>
    </div>
  );
}

function TextChannelIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" className="shrink-0 opacity-60">
      <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm-2 12H6v-2h12v2zm0-3H6V9h12v2zm0-3H6V6h12v2z"/>
    </svg>
  );
}

function VoiceChannelIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" className="shrink-0 opacity-60">
      <path d="M12 15c1.66 0 2.99-1.34 2.99-3L15 6c0-1.66-1.34-3-3-3S9 4.34 9 6v6c0 1.66 1.34 3 3 3zm5.3-3c0 3-2.54 5.1-5.3 5.1S6.7 15 6.7 12H5c0 3.42 2.72 6.23 6 6.72V22h2v-3.28c3.28-.48 6-3.3 6-6.72h-1.7z"/>
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
      <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
    </svg>
  );
}

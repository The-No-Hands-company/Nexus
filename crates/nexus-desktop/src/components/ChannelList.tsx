import { useNavigate, useParams } from "react-router-dom";
import { useStore } from "../store";
import clsx from "clsx";

export default function ChannelList() {
  const { channels, activeChannelId, setActiveChannel, activeServerId, servers } =
    useStore();
  const navigate = useNavigate();
  const { channelId } = useParams();

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

  if (!activeServerId) {
    return (
      <div className="w-60 bg-bg-800 flex flex-col shrink-0 p-3">
        <p className="text-muted text-sm">Select a server</p>
      </div>
    );
  }

  return (
    <div className="w-60 bg-bg-800 flex flex-col shrink-0 overflow-hidden">
      {/* Server header */}
      <div className="px-4 py-3 border-b border-bg-600 font-semibold text-sm text-white no-select shadow-sm flex items-center justify-between">
        <span className="truncate">{activeServer?.name ?? "Server"}</span>
      </div>

      {/* Channel list */}
      <div className="flex-1 overflow-y-auto px-2 py-2 flex flex-col gap-0.5">
        {textChannels.length > 0 && (
          <Section label="Text Channels">
            {textChannels.map((ch) => (
              <button
                key={ch.id}
                onClick={() => handleTextChannel(ch.id)}
                className={clsx(
                  "channel-item w-full text-left",
                  (activeChannelId === ch.id || channelId === ch.id) && "active"
                )}
              >
                <span className="text-muted">#</span>
                <span className="truncate text-sm">{ch.name}</span>
                {ch.isE2ee && (
                  <span className="ml-auto text-xs text-green-400" title="End-to-end encrypted">
                    🔒
                  </span>
                )}
              </button>
            ))}
          </Section>
        )}

        {voiceChannels.length > 0 && (
          <Section label="Voice Channels">
            {voiceChannels.map((ch) => (
              <button
                key={ch.id}
                onClick={() => handleVoiceChannel(ch.id)}
                className={clsx(
                  "channel-item w-full text-left",
                  (activeChannelId === ch.id || channelId === ch.id) && "active"
                )}
              >
                <VolumeIcon />
                <span className="truncate text-sm">{ch.name}</span>
              </button>
            ))}
          </Section>
        )}
      </div>
    </div>
  );
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="mb-2">
      <p className="text-xs font-semibold text-muted uppercase tracking-wider px-2 py-1 no-select">
        {label}
      </p>
      {children}
    </div>
  );
}

function VolumeIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" className="shrink-0">
      <path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3A4.5 4.5 0 0 0 14 7.97v8.05c1.48-.73 2.5-2.25 2.5-4.02z" />
    </svg>
  );
}

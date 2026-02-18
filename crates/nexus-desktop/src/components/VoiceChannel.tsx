import { useParams } from "react-router-dom";
import { useStore } from "../store";
import clsx from "clsx";

export default function VoiceChannel() {
  const { channelId } = useParams<{ channelId: string }>();
  const { channels, voiceParticipants, pttActive } = useStore();

  const channel = channels.find((c) => c.id === channelId);

  return (
    <div className="flex flex-col h-full bg-bg-800 p-6">
      <div className="mb-6">
        <div className="flex items-center gap-2 mb-1">
          <VolumeIcon />
          <h2 className="text-white font-semibold text-lg">{channel?.name}</h2>
        </div>
        <p className="text-muted text-sm">{voiceParticipants.length} participant(s)</p>
      </div>

      {/* PTT status */}
      <div
        className={clsx(
          "flex items-center gap-3 px-4 py-3 rounded-xl mb-6 transition-all",
          pttActive
            ? "bg-green-900/30 ring-1 ring-green-700"
            : "bg-bg-700"
        )}
      >
        <div className={clsx("w-3 h-3 rounded-full", pttActive ? "bg-green-400 animate-pulse" : "bg-muted")} />
        <div>
          <p className="text-sm font-medium text-white">
            {pttActive ? "Transmitting…" : "Push-to-Talk ready"}
          </p>
          <p className="text-xs text-muted">Hold CapsLock to speak</p>
        </div>
      </div>

      {/* Participants grid */}
      <div className="grid grid-cols-3 gap-3">
        {voiceParticipants.map((p) => (
          <div
            key={p.userId}
            className={clsx(
              "flex flex-col items-center gap-2 p-4 rounded-xl transition-all",
              p.speaking
                ? "bg-bg-600 ring-2 ring-green-500"
                : "bg-bg-700"
            )}
          >
            <div className="relative">
              <div className="w-14 h-14 rounded-full bg-bg-500 flex items-center justify-center text-xl font-bold text-white">
                {p.avatar ? (
                  <img src={p.avatar} alt={p.username} className="w-14 h-14 rounded-full object-cover" />
                ) : (
                  p.username[0]?.toUpperCase()
                )}
              </div>
              {p.speaking && (
                <div className="absolute -bottom-1 -right-1 w-4 h-4 bg-green-500 rounded-full border-2 border-bg-700" />
              )}
            </div>
            <p className="text-sm font-medium text-white truncate max-w-full">{p.username}</p>
            <div className="flex gap-1">
              {p.muted && <span className="text-red-400 text-xs">🎙️✗</span>}
              {p.deafened && <span className="text-red-400 text-xs">🎧✗</span>}
            </div>
          </div>
        ))}

        {voiceParticipants.length === 0 && (
          <p className="col-span-3 text-muted text-sm text-center py-8">
            No one is in this channel yet
          </p>
        )}
      </div>
    </div>
  );
}

function VolumeIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="#8892a4">
      <path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3A4.5 4.5 0 0 0 14 7.97v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06C17.52 19.71 21 16.15 21 12c0-4.15-3.48-7.71-7-8.77z" />
    </svg>
  );
}

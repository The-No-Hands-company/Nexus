/**
 * Overlay page — rendered in the transparent overlay Tauri window.
 * Shows a compact list of voice participants, designed to stay unobtrusive
 * while gaming.
 */
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore, VoiceParticipant } from "../store";

export default function OverlayPage() {
  const { voiceParticipants, setVoiceParticipants } = useStore();

  useEffect(() => {
    const unlisten = listen<VoiceParticipant[]>(
      "overlay-participants",
      (e) => setVoiceParticipants(e.payload)
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setVoiceParticipants]);

  if (voiceParticipants.length === 0) return null;

  return (
    <div
      className="p-2 rounded-lg"
      style={{
        background: "rgba(13, 15, 19, 0.82)",
        backdropFilter: "blur(8px)",
        border: "1px solid rgba(255,255,255,0.06)",
      }}
    >
      {voiceParticipants.map((p) => (
        <OverlayParticipant key={p.userId} participant={p} />
      ))}
    </div>
  );
}

function OverlayParticipant({ participant: p }: { participant: VoiceParticipant }) {
  return (
    <div
      className="flex items-center gap-2 px-1 py-0.5"
      style={{ minWidth: 160 }}
    >
      {/* Speaking indicator */}
      <div
        className="w-1.5 h-5 rounded-full transition-all"
        style={{
          background: p.speaking ? "#3ba55c" : "rgba(255,255,255,0.1)",
          boxShadow: p.speaking ? "0 0 6px #3ba55c" : "none",
        }}
      />
      {/* Avatar */}
      <div className="w-6 h-6 rounded-full bg-bg-600 flex items-center justify-center text-xs font-bold shrink-0">
        {p.avatar ? (
          <img src={p.avatar} alt={p.username} className="w-6 h-6 rounded-full" />
        ) : (
          p.username[0]?.toUpperCase()
        )}
      </div>
      {/* Name */}
      <span
        className="text-xs font-medium truncate"
        style={{ color: p.speaking ? "#ffffff" : "#8892a4" }}
      >
        {p.username}
      </span>
      {/* Muted / deafened icons */}
      <div className="flex gap-1 ml-auto">
        {p.muted && <MicOffIcon />}
        {p.deafened && <HeadphoneOffIcon />}
      </div>
    </div>
  );
}

function MicOffIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="#ed4245">
      <path d="M19 11a7 7 0 0 1-14 0H3a9 9 0 0 0 8 8.94V21H8v2h8v-2h-3v-1.06A9 9 0 0 0 21 11h-2zM12 1a4 4 0 0 0-4 4v6a4 4 0 0 0 8 0V5a4 4 0 0 0-4-4zm-2 16.93V11a6.01 6.01 0 0 1 5.06 5.33A7.001 7.001 0 0 1 10 17.93z" />
    </svg>
  );
}

function HeadphoneOffIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="#ed4245">
      <path d="M12 1C6.486 1 2 5.486 2 11v4c0 1.654 1.346 3 3 3h1c.552 0 1-.448 1-1v-5c0-.552-.448-1-1-1H4v-1c0-4.411 3.589-8 8-8s8 3.589 8 8v1h-2c-.552 0-1 .448-1 1v5c0 .552.448 1 1 1h1c1.654 0 3-1.346 3-3v-4c0-5.514-4.486-10-10-10z" />
    </svg>
  );
}

import { useEffect, useRef, useState } from "react";
import { useStore, VoiceNote } from "../store";
import { invoke } from "../invoke";

/** Inline voice-note player with waveform visualization. */
export default function VoiceNotePlayer({ note }: { note: VoiceNote }) {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [playing, setPlaying] = useState(false);
  const [progress, setProgress] = useState(0);

  const waveform: number[] = note.waveform
    ? JSON.parse(note.waveform)
    : Array(30).fill(0.3);

  const toggle = () => {
    const el = audioRef.current;
    if (!el) return;
    if (playing) {
      el.pause();
    } else {
      el.play();
    }
    setPlaying(!playing);
  };

  const formatMs = (ms: number) => {
    const s = Math.floor(ms / 1000);
    const m = Math.floor(s / 60);
    return `${m}:${String(s % 60).padStart(2, "0")}`;
  };

  return (
    <div className="flex items-center gap-2 rounded-lg bg-surface-secondary p-2 max-w-sm">
      <button
        onClick={toggle}
        className="shrink-0 w-8 h-8 rounded-full bg-accent flex items-center justify-center text-white"
        title={playing ? "Pause" : "Play"}
      >
        {playing ? "⏸" : "▶"}
      </button>

      {/* Waveform bars */}
      <div className="flex items-end gap-px h-8 flex-1 min-w-0">
        {waveform.map((amp: number, i: number) => (
          <div
            key={i}
            className="flex-1 rounded-sm transition-colors"
            style={{
              height: `${Math.max(4, amp * 32)}px`,
              backgroundColor:
                i / waveform.length < progress
                  ? "var(--color-accent)"
                  : "var(--color-text-muted)",
              opacity: i / waveform.length < progress ? 1 : 0.4,
            }}
          />
        ))}
      </div>

      <span className="text-xs text-muted shrink-0">
        {formatMs(note.durationMs)}
      </span>

      <audio
        ref={audioRef}
        src={`/api/v1/files/${note.storageKey}`}
        onTimeUpdate={() => {
          const el = audioRef.current;
          if (el && el.duration) setProgress(el.currentTime / el.duration);
        }}
        onEnded={() => {
          setPlaying(false);
          setProgress(0);
        }}
      />

      {note.transcript && (
        <p className="text-xs text-muted mt-1 line-clamp-2">{note.transcript}</p>
      )}
    </div>
  );
}

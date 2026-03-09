import { useEffect, useState } from "react";
import { useStore, Story } from "../store";
import { invoke } from "../invoke";

/** Full-screen story viewer carousel. */
export default function StoryViewer({
  stories,
  startIndex = 0,
  onClose,
}: {
  stories: Story[];
  startIndex?: number;
  onClose: () => void;
}) {
  const [idx, setIdx] = useState(startIndex);
  const current = stories[idx];

  useEffect(() => {
    if (!current) return;
    // Mark as viewed
    invoke("view_story", { id: current.id }).catch(() => {});
  }, [current?.id]);

  // Auto-advance after 6 seconds for image/text stories
  useEffect(() => {
    if (!current) return;
    if (current.mediaType === "video") return; // video controls its own playback
    const timer = setTimeout(() => {
      if (idx < stories.length - 1) setIdx(idx + 1);
      else onClose();
    }, 6000);
    return () => clearTimeout(timer);
  }, [idx, current]);

  // Keyboard navigation
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowRight" && idx < stories.length - 1) setIdx(idx + 1);
      if (e.key === "ArrowLeft" && idx > 0) setIdx(idx - 1);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [idx]);

  if (!current) return null;

  const timeLeft = () => {
    const ms = new Date(current.expiresAt).getTime() - Date.now();
    const h = Math.floor(ms / 3600000);
    return h > 0 ? `${h}h left` : "expiring soon";
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/90 flex items-center justify-center">
      {/* Progress dots */}
      <div className="absolute top-4 left-1/2 -translate-x-1/2 flex gap-1">
        {stories.map((_, i) => (
          <div
            key={i}
            className={`h-1 rounded-full transition-all ${
              i === idx ? "w-8 bg-white" : "w-4 bg-white/40"
            }`}
          />
        ))}
      </div>

      {/* Close */}
      <button
        onClick={onClose}
        className="absolute top-4 right-4 text-white text-2xl"
        title="Close"
      >
        ✕
      </button>

      {/* Content */}
      <div className="max-w-lg w-full aspect-[9/16] relative rounded-xl overflow-hidden bg-surface">
        {current.mediaType === "image" && current.storageKey && (
          <img
            src={`/api/v1/files/${current.storageKey}`}
            alt="Story"
            className="w-full h-full object-cover"
          />
        )}
        {current.mediaType === "video" && current.storageKey && (
          <video
            src={`/api/v1/files/${current.storageKey}`}
            className="w-full h-full object-cover"
            autoPlay
            onEnded={() => {
              if (idx < stories.length - 1) setIdx(idx + 1);
              else onClose();
            }}
          />
        )}
        {current.mediaType === "text" && (
          <div className="w-full h-full flex items-center justify-center p-8 bg-gradient-to-br from-indigo-600 to-purple-700">
            <p className="text-2xl text-white text-center font-semibold">
              {current.textContent}
            </p>
          </div>
        )}

        {/* Footer info */}
        <div className="absolute bottom-0 left-0 right-0 p-4 bg-gradient-to-t from-black/60 to-transparent">
          <span className="text-xs text-white/70">{timeLeft()}</span>
        </div>
      </div>

      {/* Prev / Next tap zones */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1/3 cursor-pointer"
        onClick={() => idx > 0 && setIdx(idx - 1)}
      />
      <div
        className="absolute right-0 top-0 bottom-0 w-1/3 cursor-pointer"
        onClick={() => {
          if (idx < stories.length - 1) setIdx(idx + 1);
          else onClose();
        }}
      />
    </div>
  );
}

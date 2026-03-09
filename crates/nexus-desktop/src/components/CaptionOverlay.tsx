import { useStore } from "../store";

interface Props {
  channelId: string;
  position?: "top" | "bottom";
  fontSize?: "sm" | "md" | "lg" | "xl";
}

const fontSizeClass: Record<string, string> = {
  sm: "text-xs",
  md: "text-sm",
  lg: "text-base",
  xl: "text-lg",
};

export default function CaptionOverlay({ channelId, position = "bottom", fontSize = "md" }: Props) {
  const captions = useStore((s) => s.voiceCaptions[channelId] ?? []);

  // Show only last 5 captions
  const recent = captions.slice(-5);

  if (recent.length === 0) return null;

  return (
    <div
      role="log"
      aria-live="polite"
      aria-label="Live captions"
      className={`absolute left-0 right-0 z-30 pointer-events-none px-4 py-2 flex flex-col gap-1 ${
        position === "top" ? "top-0" : "bottom-0"
      }`}
    >
      {recent.map((cap) => (
        <div
          key={cap.id}
          className={`bg-black/75 text-white rounded px-3 py-1 max-w-[80%] self-center ${fontSizeClass[fontSize] ?? "text-sm"}`}
        >
          <span className="font-semibold text-accent-400 mr-1.5">{cap.speaker_id.slice(0, 8)}:</span>
          <span>{cap.text}</span>
          {!cap.is_final && <span className="ml-1 animate-pulse text-muted">…</span>}
        </div>
      ))}
    </div>
  );
}

/**
 * StickerPicker — tabbed panel for inserting stickers into a message.
 *
 * Shows global packs + server stickers in a scrollable grid.
 * Clicking a sticker calls the provided `onSelect` callback.
 */

import { useEffect, useState } from "react";
import clsx from "clsx";
import { useStore, Sticker, StickerPack } from "../store";

interface Props {
  /** Called when the user picks a sticker. */
  onSelect: (sticker: Sticker) => void;
  onClose: () => void;
}

export default function StickerPicker({ onSelect, onClose }: Props) {
  const { stickerPacks, serverStickers, activeServerId, loadStickerPacks, loadServerStickers } =
    useStore();

  const [activeTab, setActiveTab] = useState<string>("global");

  useEffect(() => {
    loadStickerPacks();
    if (activeServerId) loadServerStickers(activeServerId);
  }, [activeServerId, loadStickerPacks, loadServerStickers]);

  // Build tabs: "global" + one per pack, then a "server" tab if applicable
  const tabs: { id: string; label: string; stickers: Sticker[] }[] = [
    ...stickerPacks.map((p) => ({
      id: `pack:${p.id}`,
      label: p.name,
      stickers: p.stickers ?? [],
    })),
  ];

  if (activeServerId) {
    const ss = serverStickers[activeServerId] ?? [];
    if (ss.length > 0) {
      tabs.push({ id: "server", label: "Server", stickers: ss });
    }
  }

  const currentStickers =
    tabs.find((t) => t.id === activeTab)?.stickers ??
    tabs[0]?.stickers ??
    [];

  if (tabs.length === 0) {
    return (
      <div className="rounded-xl bg-bg-800 p-6 text-center text-sm text-muted shadow-2xl">
        No sticker packs available.
      </div>
    );
  }

  return (
    <div className="rounded-xl bg-bg-800 shadow-2xl" style={{ width: 340 }}>
      {/* Tab bar */}
      <div className="flex overflow-x-auto border-b border-bg-600/40 px-2 pt-2 gap-1">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={clsx(
              "shrink-0 rounded-t-lg px-3 py-1.5 text-xs font-medium transition-colors",
              activeTab === tab.id
                ? "border-b-2 border-brand text-fg"
                : "text-muted hover:text-fg"
            )}
          >
            {tab.label}
          </button>
        ))}
        <button
          onClick={onClose}
          className="ml-auto flex h-7 w-7 items-center justify-center rounded text-muted hover:bg-bg-600/60 hover:text-fg"
        >
          ✕
        </button>
      </div>

      {/* Sticker grid */}
      <div className="grid grid-cols-4 gap-2 overflow-y-auto p-3" style={{ maxHeight: 260 }}>
        {currentStickers.length === 0 && (
          <p className="col-span-4 py-4 text-center text-xs text-muted">No stickers here.</p>
        )}
        {currentStickers.map((sticker) => (
          <button
            key={sticker.id}
            onClick={() => { onSelect(sticker); onClose(); }}
            title={sticker.name}
            className="aspect-square overflow-hidden rounded-lg bg-bg-700 p-1 transition-transform hover:scale-105 hover:bg-bg-600/60"
          >
            {sticker.type === "lottie" ? (
              // Lottie stickers: show static placeholder (full lottie player would need the lottie dependency)
              <div className="flex h-full w-full items-center justify-center text-2xl">🎞</div>
            ) : (
              <img
                src={sticker.assetUrl}
                alt={sticker.name}
                className="h-full w-full object-contain"
                loading="lazy"
              />
            )}
          </button>
        ))}
      </div>
    </div>
  );
}

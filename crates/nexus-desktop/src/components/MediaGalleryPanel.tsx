import { useEffect, useState } from "react";
import { useStore, Attachment } from "../store";
import { invoke } from "../invoke";

interface GalleryItem {
  id: string;
  filename: string;
  contentType: string;
  url: string | null;
  storageKey: string;
  width?: number;
  height?: number;
  createdAt: string;
}

/**
 * Media gallery panel for browsing images, videos, and files
 * within a server or channel.
 */
export default function MediaGalleryPanel({
  serverId,
  channelId,
  onClose,
}: {
  serverId?: string;
  channelId?: string;
  onClose: () => void;
}) {
  const [items, setItems] = useState<GalleryItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<"all" | "image" | "video" | "audio">("all");
  const [selectedItem, setSelectedItem] = useState<GalleryItem | null>(null);
  const { mediaGalleryFilters, loadMediaGalleryFilters } = useStore();

  useEffect(() => {
    loadMediaGalleryFilters();
  }, []);

  useEffect(() => {
    setLoading(true);
    const types =
      filter === "all"
        ? ["image", "video", "audio"]
        : [filter];
    invoke<GalleryItem[]>("browse_media", {
      serverId,
      channelId,
      mediaTypes: types,
      limit: 100,
    })
      .then(setItems)
      .catch(() => setItems([]))
      .finally(() => setLoading(false));
  }, [serverId, channelId, filter]);

  const isImage = (ct: string) => ct.startsWith("image/");
  const isVideo = (ct: string) => ct.startsWith("video/");

  return (
    <div className="flex flex-col h-full bg-surface">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-border">
        <h2 className="text-lg font-semibold">Media Gallery</h2>
        <button onClick={onClose} className="text-muted hover:text-foreground">
          ✕
        </button>
      </div>

      {/* Filter tabs */}
      <div className="flex gap-1 p-2 border-b border-border">
        {(["all", "image", "video", "audio"] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={`px-3 py-1 text-sm rounded ${
              filter === f
                ? "bg-accent text-white"
                : "bg-surface-secondary text-muted hover:bg-surface-tertiary"
            }`}
          >
            {f === "all" ? "All" : f.charAt(0).toUpperCase() + f.slice(1)}
          </button>
        ))}
      </div>

      {/* Grid */}
      <div className="flex-1 overflow-y-auto p-2">
        {loading ? (
          <p className="text-muted text-center py-8">Loading…</p>
        ) : items.length === 0 ? (
          <p className="text-muted text-center py-8">No media found</p>
        ) : (
          <div className="grid grid-cols-3 gap-2">
            {items.map((item) => (
              <button
                key={item.id}
                className="aspect-square rounded-lg overflow-hidden bg-surface-secondary hover:ring-2 ring-accent relative group"
                onClick={() => setSelectedItem(item)}
              >
                {isImage(item.contentType) ? (
                  <img
                    src={item.url || `/api/v1/files/${item.storageKey}`}
                    alt={item.filename}
                    className="w-full h-full object-cover"
                    loading="lazy"
                  />
                ) : isVideo(item.contentType) ? (
                  <div className="w-full h-full flex items-center justify-center bg-surface-tertiary">
                    <span className="text-3xl">🎬</span>
                  </div>
                ) : (
                  <div className="w-full h-full flex items-center justify-center bg-surface-tertiary">
                    <span className="text-3xl">🎵</span>
                  </div>
                )}
                {/* Overlay filename */}
                <div className="absolute bottom-0 left-0 right-0 bg-black/60 px-1 py-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                  <span className="text-[10px] text-white truncate block">
                    {item.filename}
                  </span>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Saved filters */}
      {mediaGalleryFilters.length > 0 && (
        <div className="p-2 border-t border-border">
          <p className="text-xs text-muted mb-1">Saved filters:</p>
          <div className="flex gap-1 flex-wrap">
            {mediaGalleryFilters.map((f) => (
              <span
                key={f.id}
                className="text-xs px-2 py-0.5 rounded bg-surface-secondary text-muted"
              >
                {f.name}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Lightbox */}
      {selectedItem && (
        <div
          className="fixed inset-0 z-50 bg-black/90 flex items-center justify-center"
          onClick={() => setSelectedItem(null)}
        >
          {isImage(selectedItem.contentType) ? (
            <img
              src={
                selectedItem.url ||
                `/api/v1/files/${selectedItem.storageKey}`
              }
              alt={selectedItem.filename}
              className="max-w-[90vw] max-h-[90vh] object-contain"
            />
          ) : isVideo(selectedItem.contentType) ? (
            <video
              src={`/api/v1/files/${selectedItem.storageKey}`}
              controls
              autoPlay
              className="max-w-[90vw] max-h-[90vh]"
              onClick={(e) => e.stopPropagation()}
            />
          ) : (
            <div className="text-white text-center">
              <p className="text-xl mb-2">🎵</p>
              <p>{selectedItem.filename}</p>
              <audio
                src={`/api/v1/files/${selectedItem.storageKey}`}
                controls
                autoPlay
                className="mt-4"
                onClick={(e) => e.stopPropagation()}
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

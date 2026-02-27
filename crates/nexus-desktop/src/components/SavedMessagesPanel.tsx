/**
 * SavedMessagesPanel — side panel listing the user's bookmarked messages.
 *
 * Rendered in place of the member list when `savedMessagesPanelOpen` is true.
 * Users can click a bookmark to jump to the channel, or remove it.
 */

import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useStore } from "../store";
import { formatDistanceToNow } from "date-fns";
import clsx from "clsx";

interface Props {
  onClose: () => void;
}

export default function SavedMessagesPanel({ onClose }: Props) {
  const { bookmarks, loadBookmarks, removeBookmark, setSavedMessagesPanelOpen } = useStore();
  const navigate = useNavigate();

  useEffect(() => {
    loadBookmarks();
  }, [loadBookmarks]);

  const handleClose = () => {
    setSavedMessagesPanelOpen(false);
    onClose();
  };

  const handleJump = (channelId: string) => {
    navigate(`/channels/@me/${channelId}`);
    handleClose();
  };

  const handleRemove = async (messageId: string) => {
    try {
      const { invoke } = await import("../invoke");
      await invoke("remove_bookmark", { messageId });
      removeBookmark(messageId);
    } catch (e) {
      console.error("[SavedMessages] remove error", e);
    }
  };

  return (
    <aside className="flex h-full w-80 shrink-0 flex-col border-l border-bg-600/40 bg-bg-800">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-bg-600/40 px-4 py-3">
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-widest text-muted">Saved</p>
          <p className="text-sm font-medium text-fg">Saved Messages</p>
        </div>
        <button
          onClick={handleClose}
          className="ml-3 flex h-6 w-6 items-center justify-center rounded text-muted transition-colors hover:bg-bg-600/60 hover:text-fg"
          aria-label="Close saved messages"
        >
          ✕
        </button>
      </div>

      {/* Bookmark list */}
      <div className="flex-1 overflow-y-auto px-3 py-3 space-y-2 scrollbar-thin scrollbar-thumb-bg-600/60">
        {bookmarks.length === 0 ? (
          <p className="py-12 text-center text-xs text-muted">
            No saved messages yet. Bookmark a message to see it here.
          </p>
        ) : (
          bookmarks.map((bookmark) => (
            <article
              key={bookmark.id}
              className="group rounded-lg border border-bg-600/40 bg-bg-700 p-3 hover:border-bg-600/80 transition-colors"
            >
              {/* Author + timestamp */}
              <div className="flex items-center justify-between mb-1.5">
                <span className="text-xs font-medium text-accent-400 truncate max-w-[160px]">
                  {bookmark.message?.authorUsername ?? "Unknown"}
                </span>
                <span className="text-[10px] text-muted shrink-0 ml-1">
                  {formatDistanceToNow(new Date(bookmark.createdAt), { addSuffix: true })}
                </span>
              </div>

              {/* Message preview */}
              <p className="text-sm text-fg/90 leading-snug line-clamp-3 mb-2">
                {bookmark.message?.content ?? "——"}
              </p>

              {/* Personal note if present */}
              {bookmark.note && (
                <p className="text-xs text-muted italic border-l-2 border-accent-400/40 pl-2 mb-2">
                  {bookmark.note}
                </p>
              )}

              {/* Actions */}
              <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={() => handleJump(bookmark.channelId)}
                  className={clsx(
                    "text-xs px-2 py-0.5 rounded",
                    "bg-accent-400/10 text-accent-400 hover:bg-accent-400/20 transition-colors"
                  )}
                >
                  Jump
                </button>
                <button
                  onClick={() => handleRemove(bookmark.messageId)}
                  className="text-xs px-2 py-0.5 rounded bg-dnd/10 text-dnd hover:bg-dnd/20 transition-colors"
                >
                  Remove
                </button>
              </div>
            </article>
          ))
        )}
      </div>
    </aside>
  );
}

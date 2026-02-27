/**
 * EmojiManagementPanel — upload, rename, and delete custom server emoji.
 *
 * Rendered inside the "Emoji" tab of ServerSettingsModal.
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import clsx from "clsx";

// ─── Types ────────────────────────────────────────────────────────────────────

interface Emoji {
  id: string;
  serverId: string;
  name: string;
  url: string | null;
  animated: boolean;
  managed: boolean;
  available: boolean;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Read a File as a raw byte array (number[]) suitable for Tauri. */
function readFileBytes(file: File): Promise<number[]> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const buf = e.target?.result as ArrayBuffer;
      resolve(Array.from(new Uint8Array(buf)));
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsArrayBuffer(file);
  });
}

// ─── Component ────────────────────────────────────────────────────────────────

interface Props {
  serverId: string;
}

export default function EmojiManagementPanel({ serverId }: Props) {
  const [emojis, setEmojis] = useState<Emoji[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Upload state
  const [uploadName, setUploadName] = useState("");
  const [uploadFile, setUploadFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Per-emoji rename state: Map<emojiId, pendingName>
  const [renaming, setRenaming] = useState<Record<string, string>>({});
  const [renameBusy, setRenameBusy] = useState<string | null>(null);
  const [renameError, setRenameError] = useState<string | null>(null);

  // Delete confirm
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  // ── Load ──────────────────────────────────────────────────────────────────

  const loadEmojis = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const data = await invoke<Emoji[]>("list_emoji", { serverId });
      setEmojis(data);
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }, [serverId]);

  useEffect(() => {
    loadEmojis();
  }, [loadEmojis]);

  // ── Upload ────────────────────────────────────────────────────────────────

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0] ?? null;
    setUploadFile(file);
    setUploadError(null);
    // Default the name to the file's base name (without extension) if empty
    if (file && !uploadName.trim()) {
      const baseName = file.name.replace(/\.[^.]+$/, "").replace(/[^a-zA-Z0-9_]/g, "_");
      setUploadName(baseName.slice(0, 32));
    }
  };

  const handleUpload = async () => {
    if (!uploadFile || !uploadName.trim()) return;
    setUploading(true);
    setUploadError(null);
    try {
      const fileBytes = await readFileBytes(uploadFile);
      await invoke("upload_emoji", {
        serverId,
        name: uploadName.trim(),
        fileBytes,
        contentType: uploadFile.type || "image/png",
      });
      setUploadName("");
      setUploadFile(null);
      if (fileInputRef.current) fileInputRef.current.value = "";
      await loadEmojis();
    } catch (e) {
      setUploadError(String(e));
    } finally {
      setUploading(false);
    }
  };

  // ── Rename ────────────────────────────────────────────────────────────────

  const startRename = (emoji: Emoji) => {
    setRenaming((prev) => ({ ...prev, [emoji.id]: emoji.name }));
    setRenameError(null);
  };

  const cancelRename = (id: string) => {
    setRenaming((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
    setRenameError(null);
  };

  const commitRename = async (emoji: Emoji) => {
    const newName = renaming[emoji.id]?.trim();
    if (!newName || newName === emoji.name) {
      cancelRename(emoji.id);
      return;
    }
    setRenameBusy(emoji.id);
    setRenameError(null);
    try {
      await invoke("rename_emoji", { serverId, emojiId: emoji.id, name: newName });
      cancelRename(emoji.id);
      await loadEmojis();
    } catch (e) {
      setRenameError(String(e));
    } finally {
      setRenameBusy(null);
    }
  };

  // ── Delete ────────────────────────────────────────────────────────────────

  const confirmDelete = (id: string) => {
    setConfirmDeleteId(id);
    setDeleteError(null);
  };

  const handleDelete = async () => {
    if (!confirmDeleteId) return;
    setDeleting(true);
    setDeleteError(null);
    try {
      await invoke("delete_emoji", { serverId, emojiId: confirmDeleteId });
      setConfirmDeleteId(null);
      await loadEmojis();
    } catch (e) {
      setDeleteError(String(e));
    } finally {
      setDeleting(false);
    }
  };

  const confirmEmoji = emojis.find((e) => e.id === confirmDeleteId);

  // ─────────────────────────────────────────────────────────────────────────

  return (
    <div className="space-y-6">
      {/* ── Upload section ──────────────────────────────────────────────── */}
      <div>
        <h3 className="mb-3 text-sm font-semibold text-fg">Upload Emoji</h3>
        <div className="rounded-lg border border-bg-600/50 bg-bg-700 p-4 space-y-3">
          <div className="flex flex-wrap items-end gap-3">
            {/* File picker */}
            <div>
              <label className="mb-1 block text-xs font-medium uppercase tracking-widest text-muted">
                Image
              </label>
              <label
                className={clsx(
                  "flex cursor-pointer items-center gap-2 rounded-lg border px-3 py-2 text-sm transition-colors",
                  uploadFile
                    ? "border-accent-500/60 bg-accent-600/10 text-accent-300"
                    : "border-bg-600/50 text-muted hover:border-accent-500/40 hover:text-fg"
                )}
              >
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 16 16"
                  fill="currentColor"
                  aria-hidden="true"
                >
                  <path d="M.5 9.9A.5.5 0 0 1 1 10v1a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-1a.5.5 0 0 1 1 0v1a3 3 0 0 1-3 3H3a3 3 0 0 1-3-3v-1a.5.5 0 0 1 .5-.5z" />
                  <path d="M7.646 1.146a.5.5 0 0 1 .708 0l3 3a.5.5 0 0 1-.708.708L8.5 2.707V6.5a.5.5 0 0 1-1 0V2.707L5.354 4.854a.5.5 0 1 1-.708-.708l3-3z" />
                </svg>
                {uploadFile ? uploadFile.name : "Choose file…"}
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/png,image/gif,image/webp"
                  className="sr-only"
                  onChange={handleFileChange}
                />
              </label>
              <p className="mt-0.5 text-[10px] text-muted/60">PNG / GIF / WebP · max 256 KB</p>
            </div>

            {/* Name */}
            <div className="flex-1 min-w-[140px]">
              <label className="mb-1 block text-xs font-medium uppercase tracking-widest text-muted">
                Name
              </label>
              <input
                value={uploadName}
                onChange={(e) => setUploadName(e.target.value)}
                placeholder="emoji_name"
                maxLength={32}
                aria-label="Emoji name"
                className="w-full rounded-lg border border-bg-600/50 bg-bg-600 px-3 py-2 text-sm text-fg placeholder:text-muted/60 outline-none focus:border-accent-500"
              />
              <p className="mt-0.5 text-[10px] text-muted/60">
                2–32 chars · letters, numbers, underscores
              </p>
            </div>

            {/* Preview */}
            {uploadFile && (
              <div className="flex flex-col items-center gap-1">
                <span className="text-[10px] text-muted/60">Preview</span>
                <img
                  src={URL.createObjectURL(uploadFile)}
                  alt="emoji preview"
                  className="h-10 w-10 rounded object-contain"
                />
              </div>
            )}

            <button
              onClick={handleUpload}
              disabled={uploading || !uploadFile || !uploadName.trim()}
              className="self-end rounded-lg bg-accent-600 px-4 py-2 text-sm font-medium text-white hover:bg-accent-500 disabled:opacity-40 transition-colors"
            >
              {uploading ? "Uploading…" : "Upload"}
            </button>
          </div>

          {uploadError && <p className="text-xs text-red-400">{uploadError}</p>}
        </div>
      </div>

      {/* ── Delete confirmation ────────────────────────────────────────── */}
      {confirmEmoji && (
        <div className="rounded-lg border border-red-800/50 bg-red-950/20 p-4 space-y-3">
          <p className="text-sm font-medium text-fg">
            Delete <span className="font-bold">:{confirmEmoji.name}:</span>?
          </p>
          {confirmEmoji.url && (
            <img
              src={confirmEmoji.url}
              alt={confirmEmoji.name}
              className="h-10 w-10 rounded object-contain"
            />
          )}
          <p className="text-xs text-muted">
            Removing this emoji will also remove it from any messages where it was used.
          </p>
          {deleteError && <p className="text-xs text-red-400">{deleteError}</p>}
          <div className="flex gap-2">
            <button
              onClick={handleDelete}
              disabled={deleting}
              className="rounded-lg bg-red-700 px-4 py-1.5 text-sm font-medium text-white hover:bg-red-600 disabled:opacity-40 transition-colors"
            >
              {deleting ? "Deleting…" : "Delete"}
            </button>
            <button
              onClick={() => { setConfirmDeleteId(null); setDeleteError(null); }}
              className="rounded-lg border border-bg-600/50 px-4 py-1.5 text-sm text-muted hover:text-fg transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* ── Emoji grid ────────────────────────────────────────────────── */}
      <div>
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-fg">
            Server Emoji
            <span className="ml-2 text-xs font-normal text-muted/60">{emojis.length} / 50</span>
          </h3>
          {renameError && <p className="text-xs text-red-400">{renameError}</p>}
        </div>

        {loading && <p className="text-xs text-muted py-4 text-center">Loading…</p>}
        {loadError && <p className="text-xs text-red-400 py-4 text-center">{loadError}</p>}

        {!loading && emojis.length === 0 && (
          <p className="text-sm text-muted text-center py-8">
            No custom emoji yet. Upload one above to get started.
          </p>
        )}

        {emojis.length > 0 && (
          <div className="space-y-1">
            {/* Header */}
            <div className="grid grid-cols-[40px_1fr_auto] gap-3 px-3 py-1 text-[10px] font-semibold uppercase tracking-widest text-muted/60">
              <span>Image</span>
              <span>Name</span>
              <span>Actions</span>
            </div>

            {emojis.map((emoji) => {
              const isRenaming = emoji.id in renaming;
              const isRenBusy = renameBusy === emoji.id;

              return (
                <div
                  key={emoji.id}
                  className="group grid grid-cols-[40px_1fr_auto] items-center gap-3 rounded-lg px-3 py-2 hover:bg-bg-700 transition-colors"
                >
                  {/* Image */}
                  <div className="flex items-center justify-center">
                    {emoji.url ? (
                      <img
                        src={emoji.url}
                        alt={emoji.name}
                        className="h-8 w-8 rounded object-contain"
                      />
                    ) : (
                      <div className="h-8 w-8 rounded bg-bg-600/50 flex items-center justify-center text-muted text-[10px]">
                        ?
                      </div>
                    )}
                  </div>

                  {/* Name / rename input */}
                  <div className="min-w-0">
                    {isRenaming ? (
                      <div className="flex items-center gap-1.5">
                        <span className="text-muted text-sm">:</span>
                        <input
                          autoFocus
                          value={renaming[emoji.id]}
                          onChange={(e) =>
                            setRenaming((prev) => ({ ...prev, [emoji.id]: e.target.value }))
                          }
                          onKeyDown={(e) => {
                            if (e.key === "Enter") commitRename(emoji);
                            if (e.key === "Escape") cancelRename(emoji.id);
                          }}
                          maxLength={32}
                          className="flex-1 rounded border border-accent-500/60 bg-bg-600 px-2 py-0.5 text-sm text-fg outline-none"
                        />
                        <span className="text-muted text-sm">:</span>
                        <button
                          onClick={() => commitRename(emoji)}
                          disabled={isRenBusy}
                          className="rounded px-1.5 py-0.5 text-xs text-accent-400 hover:text-accent-300 disabled:opacity-40"
                        >
                          {isRenBusy ? "…" : "Save"}
                        </button>
                        <button
                          onClick={() => cancelRename(emoji.id)}
                          className="rounded px-1.5 py-0.5 text-xs text-muted hover:text-fg"
                        >
                          Cancel
                        </button>
                      </div>
                    ) : (
                      <div className="flex items-center gap-1.5">
                        <span className="text-sm text-fg">
                          :{emoji.name}:
                        </span>
                        {emoji.animated && (
                          <span className="rounded bg-accent-600/20 px-1 py-0.5 text-[10px] text-accent-400">
                            GIF
                          </span>
                        )}
                        {emoji.managed && (
                          <span className="rounded bg-bg-600/50 px-1 py-0.5 text-[10px] text-muted">
                            managed
                          </span>
                        )}
                      </div>
                    )}
                  </div>

                  {/* Actions */}
                  {!isRenaming && !emoji.managed && (
                    <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      {/* Rename */}
                      <button
                        aria-label={`Rename :${emoji.name}:`}
                        onClick={() => startRename(emoji)}
                        className="rounded p-1 text-muted hover:text-fg transition-colors"
                      >
                        <svg
                          width="12"
                          height="12"
                          viewBox="0 0 16 16"
                          fill="currentColor"
                          aria-hidden="true"
                        >
                          <path d="M12.854.146a.5.5 0 0 0-.707 0L10.5 1.793 14.207 5.5l1.647-1.646a.5.5 0 0 0 0-.708l-3-3zm.646 6.061L9.793 2.5 3.293 9H3.5a.5.5 0 0 1 .5.5v.5h.5a.5.5 0 0 1 .5.5v.5h.5a.5.5 0 0 1 .5.5v.5h.5a.5.5 0 0 1 .5.5v.207l6.5-6.5zm-7.468 7.468A.5.5 0 0 1 6 13.5V13h-.5a.5.5 0 0 1-.5-.5V12h-.5a.5.5 0 0 1-.5-.5V11h-.5a.5.5 0 0 1-.5-.5V10h-.5a.499.499 0 0 1-.175-.032l-.179.178a.5.5 0 0 0-.11.168l-2 5a.5.5 0 0 0 .65.65l5-2a.5.5 0 0 0 .168-.11l.178-.178z" />
                        </svg>
                      </button>
                      {/* Delete */}
                      <button
                        aria-label={`Delete :${emoji.name}:`}
                        onClick={() => confirmDelete(emoji.id)}
                        className="rounded p-1 text-muted hover:text-red-400 transition-colors"
                      >
                        <svg
                          width="12"
                          height="12"
                          viewBox="0 0 16 16"
                          fill="currentColor"
                          aria-hidden="true"
                        >
                          <path d="M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6z" />
                          <path
                            fillRule="evenodd"
                            d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1v1zM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z"
                          />
                        </svg>
                      </button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

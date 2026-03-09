/**
 * FileVersionsPanel — version history for a file attachment.
 *
 * Shows all versions of a file with download/restore links.
 * Opened from the attachment context menu.
 */

import { useEffect } from "react";
import { useStore, FileVersion } from "../store";
import { invoke } from "../invoke";

interface Props {
  attachmentId: string;
  onClose: () => void;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export default function FileVersionsPanel({ attachmentId, onClose }: Props) {
  const { fileVersions, loadFileVersions, setFileVersions } = useStore();
  const versions = fileVersions[attachmentId] ?? [];

  useEffect(() => {
    loadFileVersions(attachmentId);
  }, [attachmentId, loadFileVersions]);

  const handleDelete = async (version: FileVersion) => {
    try {
      await invoke("delete_file_version", {
        attachmentId,
        version: version.versionNumber,
      });
      setFileVersions(
        attachmentId,
        versions.filter((v) => v.id !== version.id)
      );
    } catch (e) {
      console.error("delete version error", e);
    }
  };

  return (
    <aside className="flex h-full w-80 shrink-0 flex-col border-l border-bg-600/40 bg-bg-800">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-bg-600/40 px-4 py-3">
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-widest text-muted">Versions</p>
          <p className="text-sm font-medium text-fg">File History</p>
        </div>
        <button
          onClick={onClose}
          className="flex h-6 w-6 items-center justify-center rounded text-muted transition-colors hover:bg-bg-600/60 hover:text-fg"
          aria-label="Close"
        >
          ✕
        </button>
      </div>

      {/* Version list */}
      <div className="flex-1 overflow-y-auto px-4 py-2">
        {versions.length === 0 ? (
          <p className="py-8 text-center text-sm text-muted">No version history.</p>
        ) : (
          <ul className="space-y-1.5">
            {versions.map((v) => (
              <li
                key={v.id}
                className="rounded-md border border-bg-600/40 bg-bg-700/50 px-3 py-2"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-fg">
                      v{v.versionNumber} — {v.filename}
                    </p>
                    <p className="text-[10px] text-muted">
                      {formatSize(v.size)}
                      {v.contentType ? ` · ${v.contentType}` : ""}
                      {" · "}
                      {new Date(v.createdAt).toLocaleString()}
                    </p>
                    {v.comment && <p className="mt-0.5 text-xs text-muted italic">{v.comment}</p>}
                  </div>
                  <button
                    onClick={() => handleDelete(v)}
                    className="ml-2 text-xs text-muted hover:text-red-400 transition-colors"
                    title="Delete version"
                  >
                    ✕
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </aside>
  );
}

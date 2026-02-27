/**
 * ForwardModal — lets the user pick one or more channels to forward a message to.
 *
 * Shows a searchable list of server channels + DMs.
 * Max 10 targets. Calls `forward_message` on confirm.
 */

import { useState, useEffect, useRef } from "react";
import clsx from "clsx";
import { useStore, Channel, Message } from "../store";
import { invoke } from "../invoke";

interface Props {
  message: Message;
  onClose: () => void;
}

export default function ForwardModal({ message, onClose }: Props) {
  const { channels, servers, activeServerId, dmChannels } = useStore();
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Build a flat list of forwardable targets: server channels + DMs
  const serverChannels: { id: string; label: string; sub?: string }[] = channels
    .filter((c) => c.kind === "text" || c.kind === "announcement")
    .map((c) => {
      const server = servers.find((s) => s.id === c.serverId);
      return { id: c.id, label: c.name, sub: server?.name };
    });

  const dmTargets: { id: string; label: string; sub?: string }[] = dmChannels.map((dm) => ({
    id: dm.id,
    label: dm.name ?? dm.recipients.map((r) => r.username).join(", ") ?? "DM",
    sub: dm.channelType === "group_dm" ? "Group DM" : "Direct Message",
  }));

  const allTargets = [...serverChannels, ...dmTargets].filter((t) =>
    t.label.toLowerCase().includes(query.toLowerCase()) ||
    t.sub?.toLowerCase().includes(query.toLowerCase())
  );

  const toggle = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else if (next.size < 10) {
        next.add(id);
      }
      return next;
    });
  };

  const handleForward = async () => {
    if (selected.size === 0) return;
    setSending(true);
    setError(null);
    try {
      await invoke("forward_message", {
        messageId: message.id,
        targetChannelIds: Array.from(selected),
      });
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSending(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onClick={onClose}>
      <div
        className="w-[480px] max-h-[600px] flex flex-col rounded-xl bg-bg-800 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-bg-600/40 px-5 py-4">
          <h2 className="text-base font-semibold text-fg">Forward Message</h2>
          <button
            onClick={onClose}
            className="flex h-7 w-7 items-center justify-center rounded text-muted hover:bg-bg-600/60 hover:text-fg"
          >
            ✕
          </button>
        </div>

        {/* Message preview */}
        <div className="mx-5 mt-4 rounded-lg bg-bg-700 px-4 py-3 text-sm text-muted">
          <span className="font-medium text-fg">{message.authorUsername}: </span>
          {message.content.slice(0, 120)}
          {message.content.length > 120 ? "…" : ""}
        </div>

        {/* Search */}
        <div className="px-5 py-3">
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search channels or DMs…"
            className="w-full rounded-lg bg-bg-700 px-3 py-2 text-sm text-fg placeholder-muted outline-none focus:ring-1 focus:ring-brand"
          />
        </div>

        {/* Target list */}
        <div className="flex-1 overflow-y-auto px-5 pb-2">
          {allTargets.length === 0 && (
            <p className="py-4 text-center text-sm text-muted">No channels found.</p>
          )}
          {allTargets.map((t) => {
            const isSelected = selected.has(t.id);
            return (
              <button
                key={t.id}
                onClick={() => toggle(t.id)}
                className={clsx(
                  "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition-colors",
                  isSelected ? "bg-brand/20 text-fg" : "hover:bg-bg-700 text-muted hover:text-fg"
                )}
              >
                <span
                  className={clsx(
                    "flex h-5 w-5 shrink-0 items-center justify-center rounded border text-xs",
                    isSelected ? "border-brand bg-brand text-white" : "border-bg-500/60 bg-transparent"
                  )}
                >
                  {isSelected ? "✓" : ""}
                </span>
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium">{t.label}</p>
                  {t.sub && <p className="truncate text-xs text-muted">{t.sub}</p>}
                </div>
              </button>
            );
          })}
        </div>

        {/* Footer */}
        {error && <p className="px-5 text-xs text-red-400">{error}</p>}
        <div className="flex items-center justify-between border-t border-bg-600/40 px-5 py-4">
          <span className="text-xs text-muted">
            {selected.size} / 10 selected
          </span>
          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="rounded-lg bg-bg-700 px-4 py-2 text-sm text-muted hover:bg-bg-600/80 hover:text-fg"
            >
              Cancel
            </button>
            <button
              onClick={handleForward}
              disabled={selected.size === 0 || sending}
              className={clsx(
                "rounded-lg px-4 py-2 text-sm font-medium transition-opacity",
                selected.size > 0 && !sending
                  ? "bg-brand text-white hover:opacity-90"
                  : "cursor-not-allowed bg-brand/40 text-white/50"
              )}
            >
              {sending ? "Forwarding…" : "Forward"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

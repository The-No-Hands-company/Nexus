/**
 * AuditLogPanel — browse and filter the server audit log.
 *
 * Rendered inside the "Audit Log" tab of ServerSettingsModal.
 * Requires VIEW_AUDIT_LOG permission; the API enforces this, so any 403 is
 * surfaced as an error message rather than crashing the panel.
 */

import { useEffect, useState } from "react";
import { invoke } from "../invoke";
import clsx from "clsx";

// ─── Types ────────────────────────────────────────────────────────────────────

interface AuditLogEntry {
  id: string;
  server_id: string;
  actor_id: string | null;
  action: string;
  target_type: string | null;
  target_id: string | null;
  changes: Record<string, unknown>;
  reason: string | null;
  created_at: string;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

const ACTION_LABELS: Record<string, string> = {
  MEMBER_KICK:           "Member Kicked",
  MEMBER_BAN:            "Member Banned",
  MEMBER_UNBAN:          "Member Unbanned",
  MEMBER_TIMEOUT:        "Member Timed Out",
  MEMBER_TIMEOUT_REMOVE: "Timeout Removed",
  MESSAGE_DELETE:        "Message Deleted",
  CHANNEL_CREATE:        "Channel Created",
  CHANNEL_UPDATE:        "Channel Updated",
  CHANNEL_DELETE:        "Channel Deleted",
  ROLE_CREATE:           "Role Created",
  ROLE_UPDATE:           "Role Updated",
  ROLE_DELETE:           "Role Deleted",
  WEBHOOK_CREATE:        "Webhook Created",
  WEBHOOK_UPDATE:        "Webhook Updated",
  WEBHOOK_DELETE:        "Webhook Deleted",
  INVITE_CREATE:         "Invite Created",
};

/** Returns Tailwind classes for the colour-coded action badge. */
function actionBadgeClass(action: string): string {
  if (
    action.includes("DELETE") ||
    action.includes("KICK") ||
    action === "MEMBER_BAN"
  )
    return "bg-red-900/40 text-red-300 border border-red-800/50";
  if (action === "MEMBER_UNBAN" || action === "MEMBER_TIMEOUT_REMOVE")
    return "bg-green-900/40 text-green-300 border border-green-800/50";
  if (action.includes("CREATE") || action.includes("INVITE"))
    return "bg-green-900/30 text-green-300 border border-green-800/40";
  if (action.includes("UPDATE"))
    return "bg-blue-900/40 text-blue-300 border border-blue-800/50";
  if (action.includes("TIMEOUT"))
    return "bg-orange-900/40 text-orange-300 border border-orange-800/50";
  return "bg-bg-600/40 text-muted border border-bg-500/40";
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// ─── Component ───────────────────────────────────────────────────────────────

interface Props {
  serverId: string;
}

export default function AuditLogPanel({ serverId }: Props) {
  const [entries, setEntries] = useState<AuditLogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionFilter, setActionFilter] = useState("");
  const [limit, setLimit] = useState(50);

  const load = async (action: string | undefined, currentLimit: number) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<AuditLogEntry[]>("get_audit_log", {
        serverId,
        action: action || undefined,
        limit: currentLimit,
      });
      setEntries(result);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes("403") || msg.includes("MissingPermission")) {
        setError("You don't have permission to view the audit log.");
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  };

  // Initial load when the panel mounts or the server changes.
  useEffect(() => {
    setActionFilter("");
    setLimit(50);
    load(undefined, 50);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serverId]);

  const handleApplyFilter = () => {
    setLimit(50);
    load(actionFilter.trim().toUpperCase() || undefined, 50);
  };

  const handleClearFilter = () => {
    setActionFilter("");
    setLimit(50);
    load(undefined, 50);
  };

  const handleLoadMore = () => {
    const next = limit + 50;
    setLimit(next);
    load(actionFilter.trim().toUpperCase() || undefined, next);
  };

  return (
    <div className="space-y-4">
      {/* ── Filter bar ── */}
      <div className="flex gap-2">
        <input
          value={actionFilter}
          onChange={(e) => setActionFilter(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleApplyFilter()}
          placeholder="Filter by action (e.g. MEMBER_BAN)"
          className="flex-1 rounded-lg border border-bg-600/50 bg-bg-700 px-3 py-1.5 text-sm text-fg placeholder:text-muted/60 outline-none focus:border-accent-500 font-mono uppercase"
        />
        <button
          onClick={handleApplyFilter}
          disabled={loading}
          className="rounded-lg bg-accent-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-accent-500 disabled:opacity-40 transition-colors"
        >
          Apply
        </button>
        {actionFilter && (
          <button
            onClick={handleClearFilter}
            className="rounded-lg border border-bg-600/40 px-3 py-1.5 text-xs text-muted hover:text-fg transition-colors"
          >
            Clear
          </button>
        )}
        <button
          onClick={() =>
            load(actionFilter.trim().toUpperCase() || undefined, limit)
          }
          disabled={loading}
          title="Refresh"
          className="rounded-lg border border-bg-600/40 px-3 py-1.5 text-xs text-muted hover:text-fg disabled:opacity-40 transition-colors"
        >
          ↺
        </button>
      </div>

      {/* ── Content ── */}
      {loading && entries.length === 0 ? (
        <p className="text-sm text-muted text-center py-6">Loading…</p>
      ) : error ? (
        <p className="text-sm text-red-400 text-center py-6">{error}</p>
      ) : entries.length === 0 ? (
        <p className="text-sm text-muted text-center py-6">
          {actionFilter
            ? "No entries match this filter."
            : "No audit log entries yet."}
        </p>
      ) : (
        <div className="space-y-1">
          {entries.map((entry) => (
            <div
              key={entry.id}
              className="rounded-lg border border-bg-600/30 bg-bg-700/60 px-3 py-2 space-y-1"
            >
              {/* Row 1: badge + timestamp */}
              <div className="flex items-center gap-2 flex-wrap">
                <span
                  className={clsx(
                    "rounded px-1.5 py-0.5 text-[10px] font-mono font-semibold whitespace-nowrap",
                    actionBadgeClass(entry.action)
                  )}
                >
                  {ACTION_LABELS[entry.action] ?? entry.action}
                </span>
                <span className="text-[10px] text-muted ml-auto shrink-0">
                  {formatDate(entry.created_at)}
                </span>
              </div>

              {/* Row 2: actor / target / reason */}
              <div className="flex flex-wrap gap-x-4 gap-y-0.5 text-xs text-muted">
                {entry.actor_id && (
                  <span>
                    By{" "}
                    <span className="font-mono text-fg/70">
                      {entry.actor_id.slice(0, 8)}…
                    </span>
                  </span>
                )}
                {entry.target_id && (
                  <span>
                    {entry.target_type ? `${entry.target_type} ` : ""}
                    <span className="font-mono text-fg/70">
                      {entry.target_id.slice(0, 8)}…
                    </span>
                  </span>
                )}
                {entry.reason && (
                  <span className="italic text-muted/80">
                    "{entry.reason}"
                  </span>
                )}
              </div>

              {/* Row 3: collapsible changes JSON */}
              {entry.changes &&
                typeof entry.changes === "object" &&
                Object.keys(entry.changes).length > 0 && (
                  <details className="text-[10px]">
                    <summary className="cursor-pointer select-none text-muted/60 hover:text-muted transition-colors">
                      Details
                    </summary>
                    <pre className="mt-1 overflow-x-auto rounded bg-bg-600/40 px-2 py-1 text-fg/70 leading-relaxed">
                      {JSON.stringify(entry.changes, null, 2)}
                    </pre>
                  </details>
                )}
            </div>
          ))}

          {/* Load more */}
          {entries.length >= limit && (
            <button
              onClick={handleLoadMore}
              disabled={loading}
              className="w-full rounded-lg border border-bg-600/40 px-4 py-2 text-xs text-muted hover:text-fg hover:border-bg-500/60 disabled:opacity-40 transition-colors"
            >
              {loading ? "Loading…" : "Load 50 more…"}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

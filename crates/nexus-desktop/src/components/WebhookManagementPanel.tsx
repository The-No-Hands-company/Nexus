/**
 * WebhookManagementPanel — create, list, copy URL, and delete server webhooks.
 *
 * Rendered inside the "Webhooks" tab of ServerSettingsModal.
 */

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../store";
import clsx from "clsx";

// ─── Types ────────────────────────────────────────────────────────────────────

type WebhookType = "incoming" | "outgoing";

interface Webhook {
  id: string;
  webhookType: WebhookType;
  serverId: string | null;
  channelId: string | null;
  name: string;
  avatar: string | null;
  token: string | null;
  url: string | null;
  events: string[];
  active: boolean;
  deliveryCount: number;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function execUrl(hook: Webhook, baseUrl: string): string | null {
  if (hook.webhookType !== "incoming") return null;
  if (hook.url) return hook.url;
  if (hook.token) return `${baseUrl}/api/v1/webhooks/${hook.id}/${hook.token}`;
  return null;
}

// ─── Component ────────────────────────────────────────────────────────────────

interface Props {
  serverId: string;
}

export default function WebhookManagementPanel({ serverId }: Props) {
  const { channels } = useStore();
  const serverChannels = channels.filter(
    (c) => c.serverId === serverId && c.kind === "text"
  );

  const [webhooks, setWebhooks] = useState<Webhook[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Create form state
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newChannelId, setNewChannelId] = useState<string>("");
  const [createBusy, setCreateBusy] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [newlyCreated, setNewlyCreated] = useState<Webhook | null>(null);

  // Delete confirm
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  // Copy feedback
  const [copiedId, setCopiedId] = useState<string | null>(null);

  // ── Load ──────────────────────────────────────────────────────────────────

  const loadWebhooks = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const data = await invoke<Webhook[]>("list_webhooks", { serverId });
      setWebhooks(data);
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }, [serverId]);

  useEffect(() => {
    loadWebhooks();
  }, [loadWebhooks]);

  // Set default channel when channels load
  useEffect(() => {
    if (!newChannelId && serverChannels.length > 0) {
      setNewChannelId(serverChannels[0].id);
    }
  }, [serverChannels, newChannelId]);

  // ── Create ────────────────────────────────────────────────────────────────

  const handleCreate = async () => {
    if (!newName.trim() || !newChannelId) return;
    setCreateBusy(true);
    setCreateError(null);
    try {
      const wh = await invoke<Webhook>("create_incoming_webhook", {
        channelId: newChannelId,
        name: newName.trim(),
      });
      setNewlyCreated(wh);
      setCreating(false);
      setNewName("");
      await loadWebhooks();
    } catch (e) {
      setCreateError(String(e));
    } finally {
      setCreateBusy(false);
    }
  };

  const cancelCreate = () => {
    setCreating(false);
    setNewName("");
    setCreateError(null);
  };

  // ── Copy ──────────────────────────────────────────────────────────────────

  const copyUrl = async (url: string, id: string) => {
    try {
      await navigator.clipboard.writeText(url);
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      // fallback: show URL in alert
      window.alert(url);
    }
  };

  // ── Delete ────────────────────────────────────────────────────────────────

  const handleDelete = async () => {
    if (!confirmDeleteId) return;
    setDeleting(true);
    setDeleteError(null);
    try {
      await invoke("delete_webhook", { webhookId: confirmDeleteId });
      setConfirmDeleteId(null);
      if (newlyCreated?.id === confirmDeleteId) setNewlyCreated(null);
      await loadWebhooks();
    } catch (e) {
      setDeleteError(String(e));
    } finally {
      setDeleting(false);
    }
  };

  // ── Derived ───────────────────────────────────────────────────────────────

  const confirmHook = webhooks.find((w) => w.id === confirmDeleteId) ?? null;

  // Derive base URL from the current API session (fallback to location.origin)
  const baseUrl = window.location.origin;

  // ─────────────────────────────────────────────────────────────────────────

  return (
    <div className="space-y-5">
      {/* ── Newly-created token reveal ────────────────────────────────── */}
      {newlyCreated && (() => {
        const url = execUrl(newlyCreated, baseUrl);
        return (
          <div className="rounded-lg border border-green-700/50 bg-green-900/15 p-4 space-y-2">
            <p className="text-sm font-medium text-green-300">
              ✓ Webhook "{newlyCreated.name}" created
            </p>
            {url && (
              <>
                <p className="text-xs text-muted">
                  Copy this URL now — the token hidden after you leave this page.
                </p>
                <div className="flex items-center gap-2 rounded border border-green-700/40 bg-bg-800 px-3 py-2">
                  <code className="flex-1 break-all text-xs text-green-200">{url}</code>
                  <button
                    onClick={() => copyUrl(url, "new")}
                    className="shrink-0 rounded px-2 py-0.5 text-xs text-green-400 hover:text-green-300 transition-colors"
                  >
                    {copiedId === "new" ? "Copied!" : "Copy"}
                  </button>
                </div>
              </>
            )}
            <button
              onClick={() => setNewlyCreated(null)}
              className="text-xs text-muted hover:text-fg transition-colors"
            >
              Dismiss
            </button>
          </div>
        );
      })()}

      {/* ── Delete confirmation ───────────────────────────────────────── */}
      {confirmHook && (
        <div className="rounded-lg border border-red-800/50 bg-red-950/20 p-4 space-y-3">
          <p className="text-sm font-medium text-fg">
            Delete webhook "{confirmHook.name}"?
          </p>
          <p className="text-xs text-muted">
            Any integrations using this webhook will stop working immediately.
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

      {/* ── Create form ────────────────────────────────────────────────── */}
      {creating ? (
        <div className="rounded-lg border border-bg-600/50 bg-bg-700 p-4 space-y-3">
          <p className="text-sm font-semibold text-fg">New Incoming Webhook</p>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="mb-1 block text-xs font-medium uppercase tracking-widest text-muted">
                Name
              </label>
              <input
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") handleCreate(); if (e.key === "Escape") cancelCreate(); }}
                placeholder="My Webhook"
                maxLength={80}
                className="w-full rounded-lg border border-bg-600/50 bg-bg-600 px-3 py-2 text-sm text-fg placeholder:text-muted/60 outline-none focus:border-accent-500"
              />
            </div>

            <div>
              <label className="mb-1 block text-xs font-medium uppercase tracking-widest text-muted">
                Channel
              </label>
              <select
                value={newChannelId}
                onChange={(e) => setNewChannelId(e.target.value)}
                className="w-full rounded-lg border border-bg-600/50 bg-bg-600 px-3 py-2 text-sm text-fg outline-none focus:border-accent-500"
              >
                {serverChannels.length === 0 && (
                  <option value="">No text channels</option>
                )}
                {serverChannels.map((ch) => (
                  <option key={ch.id} value={ch.id}>
                    #{ch.name}
                  </option>
                ))}
              </select>
            </div>
          </div>

          {createError && <p className="text-xs text-red-400">{createError}</p>}

          <div className="flex gap-2">
            <button
              onClick={handleCreate}
              disabled={createBusy || !newName.trim() || !newChannelId}
              className="rounded-lg bg-accent-600 px-4 py-1.5 text-sm font-medium text-white hover:bg-accent-500 disabled:opacity-40 transition-colors"
            >
              {createBusy ? "Creating…" : "Create Webhook"}
            </button>
            <button onClick={cancelCreate} className="rounded-lg border border-bg-600/50 px-4 py-1.5 text-sm text-muted hover:text-fg transition-colors">
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <button
          onClick={() => { setCreating(true); setNewlyCreated(null); }}
          className="rounded-lg bg-accent-600 px-4 py-2 text-sm font-medium text-white hover:bg-accent-500 transition-colors"
        >
          + Create Webhook
        </button>
      )}

      {/* ── Webhook list ──────────────────────────────────────────────── */}
      {loading && <p className="text-xs text-muted py-4 text-center">Loading…</p>}
      {loadError && <p className="text-xs text-red-400 py-4">{loadError}</p>}

      {!loading && webhooks.length === 0 && (
        <p className="text-sm text-muted text-center py-8">
          No webhooks yet. Create one above to start receiving integrations.
        </p>
      )}

      {webhooks.length > 0 && (
        <div className="space-y-2">
          {webhooks.map((hook) => {
            const url = execUrl(hook, baseUrl);
            const channelName =
              hook.channelId
                ? serverChannels.find((c) => c.id === hook.channelId)?.name ?? "unknown"
                : null;

            return (
              <div
                key={hook.id}
                className="rounded-lg border border-bg-600/40 bg-bg-700 p-3 space-y-2"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-medium text-sm text-fg">{hook.name}</span>
                      <span
                        className={clsx(
                          "rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase",
                          hook.webhookType === "incoming"
                            ? "bg-accent-600/20 text-accent-300"
                            : "bg-yellow-900/30 text-yellow-400"
                        )}
                      >
                        {hook.webhookType}
                      </span>
                      {!hook.active && (
                        <span className="rounded bg-red-900/30 px-1.5 py-0.5 text-[10px] text-red-400">
                          disabled
                        </span>
                      )}
                    </div>
                    {channelName && (
                      <p className="text-xs text-muted mt-0.5">→ #{channelName}</p>
                    )}
                    {hook.url && hook.webhookType === "outgoing" && (
                      <p className="text-xs text-muted mt-0.5 truncate max-w-xs">{hook.url}</p>
                    )}
                    {hook.events.length > 0 && (
                      <p className="text-[11px] text-muted/60 mt-0.5">
                        Events: {hook.events.join(", ")}
                      </p>
                    )}
                    <p className="text-[11px] text-muted/60 mt-0.5">
                      {hook.deliveryCount} deliveries
                    </p>
                  </div>

                  <button
                    aria-label={`Delete webhook ${hook.name}`}
                    onClick={() => setConfirmDeleteId(hook.id)}
                    className="shrink-0 rounded p-1.5 text-muted hover:text-red-400 transition-colors"
                  >
                    <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                      <path d="M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6z"/>
                      <path fillRule="evenodd" d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1v1zM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z"/>
                    </svg>
                  </button>
                </div>

                {/* Execution URL for incoming webhooks (if token is available) */}
                {url && (
                  <div className="flex items-center gap-2 rounded border border-bg-600/30 bg-bg-800 px-2.5 py-1.5">
                    <code className="flex-1 text-[11px] text-muted truncate">{url}</code>
                    <button
                      onClick={() => copyUrl(url, hook.id)}
                      className="shrink-0 text-xs text-accent-400 hover:text-accent-300 transition-colors"
                    >
                      {copiedId === hook.id ? "Copied!" : "Copy URL"}
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

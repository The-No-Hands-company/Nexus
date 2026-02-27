/**
 * BotManagementPanel — install, list, and revoke server bots.
 *
 * Rendered inside the "Bots" tab of ServerSettingsModal.
 */

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import clsx from "clsx";

// ─── Types ────────────────────────────────────────────────────────────────────

interface BotInstall {
  id: string;
  botId: string;
  serverId: string;
  installedBy: string;
  scopes: string[];
  permissions: number;
  installedAt: string;
}

interface PublicBotInfo {
  id: string;
  name: string;
  description: string | null;
  avatar: string | null;
  verified: boolean;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function isUuid(s: string): boolean {
  return UUID_RE.test(s.trim());
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return iso;
  }
}

// ─── Component ────────────────────────────────────────────────────────────────

interface Props {
  serverId: string;
}

export default function BotManagementPanel({ serverId }: Props) {
  const [installs, setInstalls] = useState<BotInstall[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Resolved bot info cache: botId → PublicBotInfo (null = not found/not public)
  const [botInfo, setBotInfo] = useState<Record<string, PublicBotInfo | null>>({});

  // Install form
  const [showInstall, setShowInstall] = useState(false);
  const [inputBotId, setInputBotId] = useState("");
  const [lookupBusy, setLookupBusy] = useState(false);
  const [lookedUp, setLookedUp] = useState<PublicBotInfo | null>(null);
  const [lookupError, setLookupError] = useState<string | null>(null);
  const [installBusy, setInstallBusy] = useState(false);
  const [installError, setInstallError] = useState<string | null>(null);

  // Revoke confirm
  const [confirmRevokeId, setConfirmRevokeId] = useState<string | null>(null);
  const [revoking, setRevoking] = useState(false);
  const [revokeError, setRevokeError] = useState<string | null>(null);

  // ── Load ──────────────────────────────────────────────────────────────────

  const loadInstalls = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const data = await invoke<BotInstall[]>("list_server_bots", { serverId });
      setInstalls(data);

      // Resolve public info for each installed bot (best-effort, parallel)
      const unresolved = data
        .map((i) => i.botId)
        .filter((id) => !(id in botInfo));

      if (unresolved.length > 0) {
        const results = await Promise.allSettled(
          unresolved.map((id) =>
            invoke<PublicBotInfo | null>("get_public_bot_info", {
              botId: id,
            })
          )
        );
        const updates: Record<string, PublicBotInfo | null> = {};
        unresolved.forEach((id, i) => {
          const r = results[i];
          updates[id] = r.status === "fulfilled" ? r.value : null;
        });
        setBotInfo((prev) => ({ ...prev, ...updates }));
      }
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serverId]);

  useEffect(() => {
    loadInstalls();
  }, [loadInstalls]);

  // ── Lookup ────────────────────────────────────────────────────────────────

  const handleLookup = async () => {
    if (!isUuid(inputBotId)) {
      setLookupError("Enter a valid UUID (e.g. 550e8400-e29b-41d4-a716-446655440000).");
      return;
    }
    setLookupBusy(true);
    setLookupError(null);
    setLookedUp(null);
    setInstallError(null);
    try {
      const info = await invoke<PublicBotInfo | null>("get_public_bot_info", {
        botId: inputBotId.trim(),
      });
      if (!info) {
        setLookupError("Bot not found or is not publicly listed.");
      } else {
        setLookedUp(info);
      }
    } catch (e) {
      setLookupError(String(e));
    } finally {
      setLookupBusy(false);
    }
  };

  // ── Install ───────────────────────────────────────────────────────────────

  const handleInstall = async () => {
    if (!isUuid(inputBotId)) return;
    setInstallBusy(true);
    setInstallError(null);
    try {
      await invoke<BotInstall>("install_bot", {
        serverId,
        botId: inputBotId.trim(),
        permissions: null,
      });
      setShowInstall(false);
      setInputBotId("");
      setLookedUp(null);
      await loadInstalls();
    } catch (e) {
      setInstallError(String(e));
    } finally {
      setInstallBusy(false);
    }
  };

  const cancelInstall = () => {
    setShowInstall(false);
    setInputBotId("");
    setLookedUp(null);
    setLookupError(null);
    setInstallError(null);
  };

  // ── Revoke ────────────────────────────────────────────────────────────────

  const handleRevoke = async () => {
    if (!confirmRevokeId) return;
    const install = installs.find((i) => i.id === confirmRevokeId);
    if (!install) return;

    setRevoking(true);
    setRevokeError(null);
    try {
      await invoke("uninstall_bot", {
        serverId,
        botId: install.botId,
      });
      setConfirmRevokeId(null);
      await loadInstalls();
    } catch (e) {
      setRevokeError(String(e));
    } finally {
      setRevoking(false);
    }
  };

  // ── Derived ───────────────────────────────────────────────────────────────

  const revokeInstall = installs.find((i) => i.id === confirmRevokeId) ?? null;
  const revokeInfo = revokeInstall ? botInfo[revokeInstall.botId] : null;

  // ─────────────────────────────────────────────────────────────────────────

  return (
    <div className="space-y-5">
      {/* ── Revoke confirmation ────────────────────────────────────────── */}
      {revokeInstall && (
        <div className="rounded-lg border border-red-800/50 bg-red-950/20 p-4 space-y-3">
          <p className="text-sm font-medium text-fg">
            Revoke bot "{revokeInfo?.name ?? revokeInstall.botId}"?
          </p>
          <p className="text-xs text-muted">
            The bot will lose access to this server immediately. You can re-add it at any time.
          </p>
          {revokeError && <p className="text-xs text-red-400">{revokeError}</p>}
          <div className="flex gap-2">
            <button
              onClick={handleRevoke}
              disabled={revoking}
              className="rounded-lg bg-red-700 px-4 py-1.5 text-sm font-medium text-white hover:bg-red-600 disabled:opacity-40 transition-colors"
            >
              {revoking ? "Revoking…" : "Revoke Access"}
            </button>
            <button
              onClick={() => { setConfirmRevokeId(null); setRevokeError(null); }}
              className="rounded-lg border border-bg-600/50 px-4 py-1.5 text-sm text-muted hover:text-fg transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* ── Install form ──────────────────────────────────────────────── */}
      {showInstall ? (
        <div className="rounded-lg border border-bg-600/50 bg-bg-700 p-4 space-y-3">
          <p className="text-sm font-semibold text-fg">Add a Bot</p>
          <p className="text-xs text-muted">
            Enter the bot application's UUID. The bot developer provides this ID.
          </p>

          <div className="flex gap-2">
            <input
              autoFocus
              value={inputBotId}
              onChange={(e) => { setInputBotId(e.target.value); setLookedUp(null); setLookupError(null); }}
              onKeyDown={(e) => { if (e.key === "Enter") handleLookup(); if (e.key === "Escape") cancelInstall(); }}
              placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
              className="flex-1 rounded-lg border border-bg-600/50 bg-bg-600 px-3 py-2 text-sm text-fg placeholder:text-muted/50 font-mono outline-none focus:border-accent-500"
            />
            <button
              onClick={handleLookup}
              disabled={lookupBusy || !inputBotId.trim()}
              className="rounded-lg bg-bg-600 border border-bg-500/50 px-4 py-2 text-sm text-fg hover:bg-bg-500 disabled:opacity-40 transition-colors"
            >
              {lookupBusy ? "…" : "Lookup"}
            </button>
          </div>

          {lookupError && <p className="text-xs text-red-400">{lookupError}</p>}

          {/* Bot preview card */}
          {lookedUp && (
            <div className="rounded-lg border border-accent-600/30 bg-bg-800 p-3 flex gap-3 items-start">
              {lookedUp.avatar ? (
                <img
                  src={lookedUp.avatar}
                  alt={lookedUp.name}
                  className="w-10 h-10 rounded-full object-cover shrink-0"
                />
              ) : (
                <div className="w-10 h-10 rounded-full bg-bg-600 flex items-center justify-center shrink-0">
                  <span className="text-lg font-bold text-muted">
                    {lookedUp.name.charAt(0).toUpperCase()}
                  </span>
                </div>
              )}
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-sm text-fg">{lookedUp.name}</span>
                  {lookedUp.verified && (
                    <span className="rounded bg-accent-600/20 px-1.5 py-0.5 text-[10px] font-semibold text-accent-300">
                      Verified
                    </span>
                  )}
                </div>
                {lookedUp.description && (
                  <p className="text-xs text-muted mt-0.5 line-clamp-2">{lookedUp.description}</p>
                )}
              </div>
            </div>
          )}

          {installError && <p className="text-xs text-red-400">{installError}</p>}

          <div className="flex gap-2">
            <button
              onClick={handleInstall}
              disabled={installBusy || !isUuid(inputBotId)}
              className="rounded-lg bg-accent-600 px-4 py-1.5 text-sm font-medium text-white hover:bg-accent-500 disabled:opacity-40 transition-colors"
            >
              {installBusy ? "Installing…" : "Add to Server"}
            </button>
            <button onClick={cancelInstall} className="rounded-lg border border-bg-600/50 px-4 py-1.5 text-sm text-muted hover:text-fg transition-colors">
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <button
          onClick={() => setShowInstall(true)}
          className="rounded-lg bg-accent-600 px-4 py-2 text-sm font-medium text-white hover:bg-accent-500 transition-colors"
        >
          + Add Bot
        </button>
      )}

      {/* ── Bot list ──────────────────────────────────────────────────── */}
      {loading && <p className="text-xs text-muted py-4 text-center">Loading…</p>}
      {loadError && <p className="text-xs text-red-400 py-4">{loadError}</p>}

      {!loading && installs.length === 0 && (
        <p className="text-sm text-muted text-center py-8">
          No bots installed. Add a bot above to extend this server's capabilities.
        </p>
      )}

      {installs.length > 0 && (
        <div className="space-y-2">
          {installs.map((install) => {
            const info = botInfo[install.botId];

            return (
              <div
                key={install.id}
                className="rounded-lg border border-bg-600/40 bg-bg-700 p-3 flex items-start justify-between gap-3"
              >
                <div className="flex items-start gap-3 min-w-0">
                  {/* Avatar */}
                  {info?.avatar ? (
                    <img
                      src={info.avatar}
                      alt={info.name}
                      className="w-10 h-10 rounded-full object-cover shrink-0"
                    />
                  ) : (
                    <div className="w-10 h-10 rounded-full bg-bg-600 flex items-center justify-center shrink-0">
                      <span className="text-base font-bold text-muted">
                        {(info?.name ?? install.botId).charAt(0).toUpperCase()}
                      </span>
                    </div>
                  )}

                  {/* Info */}
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-medium text-sm text-fg">
                        {info?.name ?? <code className="text-xs text-muted">{install.botId}</code>}
                      </span>
                      {info?.verified && (
                        <span className="rounded bg-accent-600/20 px-1.5 py-0.5 text-[10px] font-semibold text-accent-300">
                          Verified
                        </span>
                      )}
                    </div>

                    {info?.description && (
                      <p className="text-xs text-muted mt-0.5 line-clamp-1">{info.description}</p>
                    )}

                    {/* Scopes */}
                    {install.scopes.length > 0 && (
                      <div className="flex flex-wrap gap-1 mt-1">
                        {install.scopes.map((scope) => (
                          <span
                            key={scope}
                            className="rounded bg-bg-600 px-1.5 py-0.5 text-[10px] text-muted"
                          >
                            {scope}
                          </span>
                        ))}
                      </div>
                    )}

                    <p className="text-[11px] text-muted/60 mt-1">
                      Added {formatDate(install.installedAt)}
                    </p>
                  </div>
                </div>

                {/* Revoke button */}
                <button
                  aria-label={`Revoke bot ${info?.name ?? install.botId}`}
                  onClick={() => setConfirmRevokeId(install.id)}
                  className={clsx(
                    "shrink-0 rounded px-3 py-1 text-xs border transition-colors",
                    "border-red-800/50 text-red-400 hover:bg-red-900/20"
                  )}
                >
                  Revoke
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

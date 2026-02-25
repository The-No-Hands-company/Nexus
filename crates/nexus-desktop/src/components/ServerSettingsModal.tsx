import { useState, useEffect, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "../invoke";
import { useStore, Server } from "../store";
import clsx from "clsx";

interface Invite {
  code: string;
  serverId: string;
  maxUses: number | null;
  uses: number;
  expiresAt: string | null;
  createdAt: string;
}

type Tab = "overview" | "invites" | "danger";

interface Props {
  server: Server;
  onClose: () => void;
}

export default function ServerSettingsModal({ server, onClose }: Props) {
  const navigate = useNavigate();
  const { loadServers, setActiveChannel } = useStore();
  const [tab, setTab] = useState<Tab>("overview");

  // Overview state
  const [name, setName] = useState(server.name);
  const [isPublic, setIsPublic] = useState(false); // loaded from server details
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState(false);

  // Invites state
  const [invites, setInvites] = useState<Invite[]>([]);
  const [invitesLoading, setInvitesLoading] = useState(false);
  const [newCode, setNewCode] = useState<string | null>(null);
  const [copied, setCopied] = useState("");

  // Danger state
  const [deleteConfirm, setDeleteConfirm] = useState("");
  const [deleting, setDeleting] = useState(false);

  // Load invites when switching to invites tab
  useEffect(() => {
    if (tab !== "invites") return;
    setInvitesLoading(true);
    invoke<Invite[]>("list_server_invites", { serverId: server.id })
      .then(setInvites)
      .catch(() => setInvites([]))
      .finally(() => setInvitesLoading(false));
  }, [tab, server.id]);

  const handleSave = async () => {
    if (!name.trim()) return;
    setSaving(true);
    setSaveError(null);
    setSaveOk(false);
    try {
      await invoke("update_server", { serverId: server.id, name: name.trim() });
      await loadServers();
      setSaveOk(true);
      setTimeout(() => setSaveOk(false), 2000);
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleCreateInvite = async () => {
    try {
      const result = await invoke<{ code: string }>("create_invite", {
        serverId: server.id,
        maxUses: null,
        maxAgeSecs: null,
      });
      setNewCode(result.code);
      // Refresh list
      const updated = await invoke<Invite[]>("list_server_invites", {
        serverId: server.id,
      });
      setInvites(updated);
    } catch (e) {
      console.error("create invite error", e);
    }
  };

  const copyInviteLink = useCallback(
    (code: string) => {
      const link = `${window.location.origin}/invite/${code}`;
      navigator.clipboard.writeText(link).catch(() => {});
      setCopied(code);
      setTimeout(() => setCopied(""), 2000);
    },
    []
  );

  const handleDelete = async () => {
    if (deleteConfirm !== server.name) return;
    setDeleting(true);
    try {
      await invoke("delete_server", { serverId: server.id });
      await loadServers();
      setActiveChannel(null);
      navigate("/");
      onClose();
    } catch (e) {
      console.error("delete server error", e);
      setDeleting(false);
    }
  };

  const handleBackdrop = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) onClose();
    },
    [onClose]
  );

  const TABS: { id: Tab; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "invites", label: "Invites" },
    { id: "danger", label: "Danger Zone" },
  ];

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={handleBackdrop}
    >
      <div className="flex h-[520px] w-[700px] rounded-xl bg-bg-800 shadow-2xl overflow-hidden">
        {/* Sidebar */}
        <div className="w-48 shrink-0 bg-bg-700 border-r border-bg-600/40 flex flex-col p-3 gap-0.5">
          <p className="px-2 py-1 text-[10px] font-semibold uppercase tracking-widest text-muted/70 truncate mb-1">
            {server.name}
          </p>
          {TABS.map((t) => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={clsx(
                "w-full text-left rounded px-2 py-1.5 text-sm transition-colors",
                tab === t.id
                  ? "bg-bg-600/60 text-fg font-medium"
                  : "text-muted hover:bg-bg-600/40 hover:text-fg",
                t.id === "danger" && "mt-auto text-red-400 hover:text-red-300"
              )}
            >
              {t.label}
            </button>
          ))}
          <button
            onClick={onClose}
            className="mt-2 w-full rounded px-2 py-1.5 text-xs text-muted hover:text-fg hover:bg-bg-600/40 transition-colors text-left"
          >
            ✕ Close
          </button>
        </div>

        {/* Content pane */}
        <div className="flex-1 overflow-y-auto p-6">
          {/* ── Overview ── */}
          {tab === "overview" && (
            <div className="space-y-5">
              <h2 className="text-lg font-semibold text-fg">Overview</h2>

              <div>
                <label className="block text-xs font-medium uppercase tracking-widest text-muted mb-1">
                  Server Name
                </label>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  maxLength={100}
                  className="w-full rounded-lg border border-bg-600/50 bg-bg-700 px-3 py-2 text-sm text-fg placeholder:text-muted/60 outline-none focus:border-accent-500"
                />
              </div>

              <div className="flex items-center gap-3">
                <button
                  onClick={() => setIsPublic((v) => !v)}
                  className={clsx(
                    "relative h-5 w-9 rounded-full transition-colors",
                    isPublic ? "bg-accent-600" : "bg-bg-600/60"
                  )}
                >
                  <span
                    className={clsx(
                      "absolute top-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform",
                      isPublic ? "translate-x-4" : "translate-x-0.5"
                    )}
                  />
                </button>
                <span className="text-sm text-fg">
                  Public server{" "}
                  <span className="text-xs text-muted">(discoverable in server browser)</span>
                </span>
              </div>

              {saveError && (
                <p className="text-xs text-red-400">{saveError}</p>
              )}
              {saveOk && (
                <p className="text-xs text-green-400">✓ Changes saved</p>
              )}

              <button
                onClick={handleSave}
                disabled={saving || !name.trim()}
                className="rounded-lg bg-accent-600 px-5 py-2 text-sm font-medium text-white hover:bg-accent-500 disabled:opacity-40 transition-colors"
              >
                {saving ? "Saving…" : "Save Changes"}
              </button>
            </div>
          )}

          {/* ── Invites ── */}
          {tab === "invites" && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h2 className="text-lg font-semibold text-fg">Invites</h2>
                <button
                  onClick={handleCreateInvite}
                  className="rounded-lg bg-accent-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-accent-500 transition-colors"
                >
                  + Create Invite
                </button>
              </div>

              {newCode && (
                <div className="flex items-center gap-2 rounded-lg border border-green-700/50 bg-green-900/20 px-3 py-2">
                  <span className="flex-1 font-mono text-sm text-green-300">
                    {window.location.origin}/invite/{newCode}
                  </span>
                  <button
                    onClick={() => copyInviteLink(newCode)}
                    className="text-xs text-green-400 hover:text-green-300"
                  >
                    {copied === newCode ? "Copied!" : "Copy"}
                  </button>
                </div>
              )}

              {invitesLoading ? (
                <p className="text-sm text-muted text-center py-6">Loading invites…</p>
              ) : invites.length === 0 ? (
                <p className="text-sm text-muted text-center py-6">
                  No active invites. Create one above.
                </p>
              ) : (
                <div className="space-y-2">
                  {invites.map((inv) => (
                    <div
                      key={inv.code}
                      className="flex items-center gap-3 rounded-lg border border-bg-600/40 bg-bg-700 px-3 py-2"
                    >
                      <span className="font-mono text-sm text-fg flex-1">{inv.code}</span>
                      <span className="text-xs text-muted">
                        {inv.uses}/{inv.maxUses ?? "∞"} uses
                      </span>
                      {inv.expiresAt && (
                        <span className="text-xs text-muted">
                          expires {new Date(inv.expiresAt).toLocaleDateString()}
                        </span>
                      )}
                      <button
                        onClick={() => copyInviteLink(inv.code)}
                        className="text-xs text-accent-400 hover:text-accent-300 transition-colors"
                      >
                        {copied === inv.code ? "Copied!" : "Copy"}
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* ── Danger Zone ── */}
          {tab === "danger" && (
            <div className="space-y-5">
              <h2 className="text-lg font-semibold text-red-400">Danger Zone</h2>

              <div className="rounded-lg border border-red-800/50 bg-red-950/20 p-4 space-y-3">
                <p className="text-sm font-medium text-fg">Delete this Server</p>
                <p className="text-xs text-muted leading-relaxed">
                  This action is <strong className="text-fg">permanent and cannot be undone</strong>.
                  All channels, messages, and members will be removed.
                </p>
                <label className="block text-xs font-medium text-muted mb-1">
                  Type <code className="text-red-400">{server.name}</code> to confirm
                </label>
                <input
                  value={deleteConfirm}
                  onChange={(e) => setDeleteConfirm(e.target.value)}
                  placeholder={server.name}
                  className="w-full rounded-lg border border-red-800/50 bg-bg-700 px-3 py-2 text-sm text-fg placeholder:text-muted/60 outline-none focus:border-red-500"
                />
                <button
                  onClick={handleDelete}
                  disabled={deleteConfirm !== server.name || deleting}
                  className="rounded-lg bg-red-700 px-5 py-2 text-sm font-medium text-white hover:bg-red-600 disabled:opacity-40 transition-colors"
                >
                  {deleting ? "Deleting…" : "Delete Server"}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

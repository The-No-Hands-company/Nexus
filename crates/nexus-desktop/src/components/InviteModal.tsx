import { useState, useEffect } from "react";
import { invoke } from "../invoke";

interface Props {
  serverId: string;
  serverName: string;
  onClose: () => void;
}

export default function InviteModal({ serverId, serverName, onClose }: Props) {
  const [code, setCode] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    invoke<{ code: string }>("create_invite", { serverId, maxUses: null, maxAgeSecs: null })
      .then((r) => setCode(r.code))
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [serverId]);

  const inviteUrl = code
    ? `${window.location.origin}/invite/${code}`
    : null;

  const copy = async () => {
    if (!inviteUrl) return;
    await navigator.clipboard.writeText(inviteUrl);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="bg-bg-800 rounded-xl shadow-2xl w-[420px] p-6 flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-bold text-white">Invite People</h2>
          <button
            onClick={onClose}
            className="text-muted hover:text-fg transition-colors"
          >
            ✕
          </button>
        </div>

        <p className="text-sm text-muted">
          Share this link to invite someone to <strong className="text-fg">{serverName}</strong>.
        </p>

        {loading && (
          <div className="text-sm text-muted animate-pulse">Generating invite…</div>
        )}

        {error && (
          <div className="text-sm text-red-400 bg-red-950/30 rounded p-2">{error}</div>
        )}

        {inviteUrl && (
          <div className="flex gap-2">
            <input
              readOnly
              value={inviteUrl}
              className="flex-1 bg-bg-900 border border-bg-600 rounded-lg px-3 py-2 text-sm text-fg font-mono focus:outline-none cursor-text"
              onFocus={(e) => e.target.select()}
            />
            <button
              onClick={copy}
              className="px-4 py-2 rounded-lg text-sm font-semibold transition-colors"
              style={{
                background: copied ? "var(--color-success, #22c55e)" : "var(--color-accent-500, #7c3aed)",
                color: "white",
              }}
            >
              {copied ? "Copied!" : "Copy"}
            </button>
          </div>
        )}

        {code && (
          <p className="text-xs text-muted">
            Or share just the code: <code className="font-mono bg-bg-900 px-1 rounded">{code}</code>
          </p>
        )}
      </div>
    </div>
  );
}

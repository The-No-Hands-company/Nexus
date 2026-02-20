import { useState } from "react";
import { invoke } from "../invoke";
import { useStore, Server } from "../store";
import { useNavigate } from "react-router-dom";

interface Props {
  onClose: () => void;
}

export default function JoinServerModal({ onClose }: Props) {
  const { servers, setServers, setActiveServer } = useStore();
  const navigate = useNavigate();
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Extract the code from either a raw code or a full URL
  function parseCode(raw: string): string {
    raw = raw.trim();
    // URL like http://localhost:1420/invite/abc123xy
    const match = raw.match(/\/invite\/([a-zA-Z0-9]+)/);
    if (match) return match[1];
    return raw;
  }

  const handleJoin = async (e: React.FormEvent) => {
    e.preventDefault();
    const code = parseCode(input);
    if (!code) return;
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<{ server: { id: string; name: string } }>(
        "join_via_invite",
        { code }
      );
      const joined = result.server;

      // Add to store if not already present, then navigate there
      const already = servers.find((s) => s.id === joined.id);
      if (!already) {
        const newServer: Server = {
          id: joined.id,
          name: joined.name,
          ownerId: "",
        };
        setServers([...servers, newServer]);
      }
      setActiveServer(joined.id);
      navigate("/");
      onClose();
    } catch (err) {
      setError(typeof err === "string" ? err : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="bg-bg-800 rounded-xl shadow-2xl w-[420px] p-6 flex flex-col gap-5">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-bold text-white">Join a Server</h2>
          <button onClick={onClose} className="text-muted hover:text-fg transition-colors">✕</button>
        </div>
        <p className="text-sm text-muted -mt-2">
          Enter an invite link or code to join an existing server.
        </p>

        <form onSubmit={handleJoin} className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs font-semibold text-muted uppercase tracking-wide">
              Invite Link or Code
            </label>
            <input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="http://localhost:1420/invite/abc123xy  or  abc123xy"
              autoFocus
              className="bg-bg-900 border border-bg-600 rounded-lg px-3 py-2 text-white placeholder:text-muted text-sm focus:outline-none focus:border-accent-500 transition-colors"
            />
          </div>

          {error && (
            <p className="text-sm text-red-400 bg-red-950/30 rounded p-2">{error}</p>
          )}

          <div className="flex gap-2 justify-end">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg text-sm font-semibold text-muted hover:text-fg hover:bg-bg-700 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || !input.trim()}
              className="btn-primary px-6 py-2 text-sm"
            >
              {loading ? "Joining…" : "Join Server"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

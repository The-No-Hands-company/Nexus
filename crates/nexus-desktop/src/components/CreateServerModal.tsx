import { useState } from "react";
import { invoke } from "../invoke";
import { useStore, Server } from "../store";

interface Props {
  onClose: () => void;
}

export default function CreateServerModal({ onClose }: Props) {
  const { servers, setServers, setActiveServer } = useStore();
  const [name, setName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) return;

    setLoading(true);
    setError(null);
    try {
      const server = await invoke<Server>("create_server", {
        name: trimmed,
        isPublic: false,
      });
      setServers([...servers, server]);
      setActiveServer(server.id);
      onClose();
    } catch (err) {
      setError(typeof err === "string" ? err : "Failed to create server");
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
        <h2 className="text-lg font-bold text-white">Create a Server</h2>
        <p className="text-sm text-muted -mt-2">
          Give your server a personality with a name. You can always change it later.
        </p>

        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs font-semibold text-muted uppercase tracking-wide">
              Server Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My awesome server"
              maxLength={100}
              autoFocus
              className="bg-bg-900 border border-bg-600 rounded-lg px-3 py-2 text-white placeholder:text-muted text-sm focus:outline-none focus:border-accent-500 transition-colors"
            />
          </div>

          {error && (
            <p className="text-red-400 text-xs">{error}</p>
          )}

          <div className="flex gap-3 justify-end pt-1">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg text-sm text-muted hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || !name.trim()}
              className="px-5 py-2 rounded-lg text-sm font-semibold bg-accent-500 hover:bg-accent-600 text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? "Creating…" : "Create"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

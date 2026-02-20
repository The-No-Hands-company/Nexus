import { useState, FormEvent } from "react";
import { Link } from "react-router-dom";
import { invoke } from "../invoke";
import { useStore, Session } from "../store";

interface AuthUserInfo {
  id: string;
  username: string;
  display_name?: string | null;
  avatar?: string | null;
}

interface LoginResponse {
  access_token: string;
  refresh_token: string;
  user: AuthUserInfo;
}

export default function LoginPage() {
  const { setSession } = useStore();

  const [serverUrl, setServerUrl] = useState("http://localhost:8080");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      // Persist server URL so commands know where to connect
      await invoke("set_server_url", { url: serverUrl });

      const resp = await invoke<LoginResponse>("login", { username, password });
      const session: Session = {
        userId: resp.user.id,
        username: resp.user.username,
        displayName: resp.user.display_name ?? undefined,
        avatar: resp.user.avatar ?? undefined,
        serverUrl,
        accessToken: resp.access_token,
      };
      setSession(session);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex h-full items-center justify-center bg-bg-900">
      <div className="w-full max-w-sm bg-bg-800 rounded-xl p-8 shadow-2xl">
        {/* Logo */}
        <div className="flex flex-col items-center mb-8">
          <div className="w-16 h-16 rounded-2xl bg-accent-500 flex items-center justify-center mb-3">
            <span className="text-white text-3xl font-bold select-none">N</span>
          </div>
          <h1 className="text-xl font-bold text-white">Welcome back</h1>
          <p className="text-muted text-sm mt-1">Sign in to Nexus</p>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <div>
            <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">
              Server URL
            </label>
            <input
              className="input"
              type="url"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              placeholder="https://your-nexus-server.com"
              required
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">
              Username
            </label>
            <input
              className="input"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="your_username"
              autoComplete="username"
              required
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">
              Password
            </label>
            <input
              className="input"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••"
              autoComplete="current-password"
              required
            />
          </div>

          {error && (
            <p className="text-sm text-red-400 bg-red-950/30 rounded p-2">
              {error}
            </p>
          )}

          <button
            type="submit"
            className="btn-primary mt-2 w-full"
            disabled={loading}
          >
            {loading ? "Signing in…" : "Sign In"}
          </button>
        </form>

        <p className="text-center text-sm text-muted mt-6">
          Don't have an account?{" "}
          <Link to="/register" className="text-accent-400 hover:underline">
            Create one
          </Link>
        </p>
      </div>
    </div>
  );
}

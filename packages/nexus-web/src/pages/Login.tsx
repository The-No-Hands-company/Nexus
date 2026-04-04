import { FormEvent, useState } from "react";
import { useNavigate, Link } from "react-router-dom";
import { useStore } from "../store";

export default function Login() {
  const { setSession, serverUrl, setServerUrl } = useStore();
  const navigate = useNavigate();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const res = await fetch(`${serverUrl}/api/v1/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
      const body = await res.json();
      setSession({
        accessToken: body.access_token,
        refreshToken: body.refresh_token ?? "",
        username: body.user?.username ?? username,
        userId: body.user?.id ?? "",
        avatar: body.user?.avatar ?? null,
        serverUrl,
      });
      navigate("/");
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen bg-bg-900 flex items-center justify-center p-4">
      <div className="w-full max-w-sm">
        <div className="text-center mb-8">
          <div className="w-12 h-12 rounded-xl bg-accent-500/20 flex items-center justify-center mx-auto mb-3">
            <span className="text-accent-300 font-bold text-lg">NX</span>
          </div>
          <h1 className="text-xl font-semibold text-fg">Welcome back</h1>
          <p className="text-muted text-sm mt-1">Sign in to your Nexus account</p>
        </div>

        <form onSubmit={submit} className="bg-bg-800 rounded-xl border border-bg-600/40 p-6 flex flex-col gap-4">
          <div>
            <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">
              Server URL
            </label>
            <input
              className="nx-input"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              placeholder="http://localhost:8080"
            />
          </div>
          <div>
            <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">
              Username
            </label>
            <input
              className="nx-input"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              autoFocus
              required
            />
          </div>
          <div>
            <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">
              Password
            </label>
            <input
              className="nx-input"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </div>

          {error && (
            <p className="text-sm text-red-400 bg-red-950/40 border border-red-800/50 rounded-lg px-3 py-2">
              {error}
            </p>
          )}

          <button className="nx-btn nx-btn-primary w-full" disabled={loading}>
            {loading ? "Signing in…" : "Sign In"}
          </button>
        </form>

        <p className="text-center text-muted text-sm mt-4">
          No account?{" "}
          <Link to="/register" className="text-accent-300 hover:underline">
            Create one
          </Link>
        </p>
      </div>
    </div>
  );
}

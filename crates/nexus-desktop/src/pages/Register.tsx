import { useState, FormEvent } from "react";
import { Link } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useStore, Session } from "../store";

interface AuthUserInfo {
  id: string;
  username: string;
  display_name?: string | null;
  avatar?: string | null;
}

interface AuthResponse {
  access_token: string;
  refresh_token: string;
  user: AuthUserInfo;
}

export default function RegisterPage() {
  const { setSession } = useStore();

  const [serverUrl, setServerUrl] = useState("http://localhost:8080");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);

    if (password !== confirm) {
      setError("Passwords do not match.");
      return;
    }

    setLoading(true);
    try {
      await invoke("set_server_url", { url: serverUrl });

      const resp = await invoke<AuthResponse>("register", {
        username,
        email,
        password,
      });

      const session: Session = {
        userId: resp.user.id,
        username: resp.user.username,
        serverUrl,
        accessToken: resp.access_token,
      };
      setSession(session);
      // App.tsx will redirect to "/" once session is set
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
          <h1 className="text-xl font-bold text-white">Create an account</h1>
          <p className="text-muted text-sm mt-1">Join Nexus</p>
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
              Email
            </label>
            <input
              className="input"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@example.com"
              autoComplete="email"
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
              autoComplete="new-password"
              required
            />
          </div>

          <div>
            <label className="block text-xs font-semibold text-muted uppercase tracking-wide mb-1">
              Confirm Password
            </label>
            <input
              className="input"
              type="password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              placeholder="••••••••"
              autoComplete="new-password"
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
            {loading ? "Creating account…" : "Create Account"}
          </button>
        </form>

        <p className="text-center text-sm text-muted mt-6">
          Already have an account?{" "}
          <Link to="/login" className="text-accent-400 hover:underline">
            Sign in
          </Link>
        </p>
      </div>
    </div>
  );
}

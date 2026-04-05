import { FormEvent, useState } from "react";
import { useNavigate } from "react-router-dom";
import { setSession, ApiError } from "../api";

export default function LoginPage() {
  const navigate = useNavigate();
  const [serverUrl, setServerUrl] = useState("http://localhost:8080");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const base = serverUrl.replace(/\/$/, "");
      const res = await fetch(`${base}/api/v1/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      if (!res.ok) throw new ApiError(res.status, await res.text());
      const body = await res.json();

      // Verify the user is an instance admin before granting access
      const meRes = await fetch(`${base}/api/v1/admin/overview`, {
        headers: { Authorization: `Bearer ${body.access_token}` },
      });
      if (meRes.status === 403) {
        throw new Error("This account does not have Instance Admin privileges.");
      }
      if (!meRes.ok) {
        throw new Error(`Server error (${meRes.status}) — check server URL.`);
      }

      setSession({
        accessToken: body.access_token,
        username: body.user?.username ?? username,
        userId: body.user?.id ?? "",
        serverUrl: base,
      });
      navigate("/overview");
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{
      minHeight: "100vh", display: "flex",
      alignItems: "center", justifyContent: "center",
      background: "var(--bg-900)", padding: 20,
    }}>
      <div style={{ width: "100%", maxWidth: 400 }}>
        {/* Logo */}
        <div style={{ textAlign: "center", marginBottom: 32 }}>
          <div style={{
            width: 52, height: 52, borderRadius: 14,
            background: "var(--accent-dim)",
            display: "flex", alignItems: "center", justifyContent: "center",
            color: "var(--accent)", fontWeight: 700, fontSize: 20,
            margin: "0 auto 14px",
          }}>NX</div>
          <h1 style={{ fontSize: 22, fontWeight: 700, color: "var(--fg)" }}>Nexus Admin</h1>
          <p style={{ color: "var(--muted)", fontSize: 14, marginTop: 4 }}>
            Instance administrator access only
          </p>
        </div>

        <form onSubmit={submit} className="card" style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          <div>
            <label style={labelStyle}>Server URL</label>
            <input
              style={{ width: "100%" }}
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              placeholder="https://nexus.example.com"
              required
            />
          </div>
          <div>
            <label style={labelStyle}>Username</label>
            <input
              style={{ width: "100%" }}
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              autoFocus
              required
            />
          </div>
          <div>
            <label style={labelStyle}>Password</label>
            <input
              style={{ width: "100%" }}
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </div>

          {error && (
            <div style={{
              background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.3)",
              borderRadius: 8, padding: "10px 14px", color: "#f87171", fontSize: 13,
            }}>{error}</div>
          )}

          <button
            type="submit"
            className="btn btn-primary"
            disabled={loading}
            style={{ width: "100%", justifyContent: "center", padding: "11px 0" }}
          >
            {loading ? "Signing in…" : "Sign In"}
          </button>
        </form>

        <p style={{ textAlign: "center", color: "var(--muted)", fontSize: 12, marginTop: 16 }}>
          To grant admin access: <code style={{ color: "var(--fg-dim)" }}>UPDATE users SET flags = flags | 128 WHERE username = 'you';</code>
        </p>
      </div>
    </div>
  );
}

const labelStyle: React.CSSProperties = {
  display: "block", fontSize: 11, fontWeight: 700,
  color: "var(--muted)", textTransform: "uppercase",
  letterSpacing: "0.8px", marginBottom: 6,
};

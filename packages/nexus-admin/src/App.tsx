import { useEffect, useState } from "react";
import { BrowserRouter, Routes, Route, Navigate, NavLink, useNavigate } from "react-router-dom";
import { getSession, setSession, subscribeSession, whoami, type Session } from "./api";
import OverviewPage from "./pages/Overview";
import UsersPage from "./pages/Users";
import UserDetailPage from "./pages/UserDetail";
import ServersPage from "./pages/Servers";
import FederationPage from "./pages/Federation";
import ModerationPage from "./pages/Moderation";
import AuditLogPage from "./pages/AuditLog";

// ── Auth hook ─────────────────────────────────────────────────────────────────

function useSession(): Session | null {
  const [session, setLocal] = useState<Session | null>(getSession);
  useEffect(() => subscribeSession(() => setLocal(getSession())), []);
  return session;
}

// ── Shell layout ──────────────────────────────────────────────────────────────

const NAV_ITEMS = [
  { to: "/overview",   icon: "⬛", label: "Overview" },
  { to: "/users",      icon: "👤", label: "Users" },
  { to: "/servers",    icon: "🏠", label: "Servers" },
  { to: "/moderation", icon: "🛡", label: "Moderation" },
  { to: "/federation", icon: "🌐", label: "Federation" },
  { to: "/audit",      icon: "📋", label: "Audit Log" },
];

function Shell({ children }: { children: React.ReactNode }) {
  const session = useSession();
  const navigate = useNavigate();

  function logout() {
    setSession(null);
    navigate("/login");
  }

  return (
    <div style={{ display: "flex", height: "100%" }}>
      {/* Sidebar */}
      <nav style={{
        width: "var(--sidebar-w)", flexShrink: 0,
        background: "var(--bg-800)", borderRight: "1px solid var(--border)",
        display: "flex", flexDirection: "column", padding: "0",
        overflow: "hidden",
      }}>
        {/* Logo */}
        <div style={{
          padding: "18px 20px 16px",
          borderBottom: "1px solid var(--border)",
        }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div style={{
              width: 32, height: 32, borderRadius: 8,
              background: "var(--accent-dim)",
              display: "flex", alignItems: "center", justifyContent: "center",
              color: "var(--accent)", fontWeight: 700, fontSize: 13,
            }}>NX</div>
            <div>
              <div style={{ fontWeight: 700, fontSize: 13, color: "var(--fg)" }}>Nexus Admin</div>
              <div style={{ fontSize: 11, color: "var(--muted)" }}>{session?.serverUrl?.replace(/https?:\/\//, "")}</div>
            </div>
          </div>
        </div>

        {/* Nav links */}
        <div style={{ flex: 1, padding: "8px", overflowY: "auto" }}>
          {NAV_ITEMS.map(({ to, icon, label }) => (
            <NavLink
              key={to}
              to={to}
              style={({ isActive }) => ({
                display: "flex", alignItems: "center", gap: 10,
                padding: "9px 12px", borderRadius: 8, marginBottom: 2,
                textDecoration: "none", fontSize: 13, fontWeight: 500,
                transition: "background 0.12s",
                background: isActive ? "var(--accent-dim)" : "transparent",
                color: isActive ? "var(--accent)" : "var(--fg-dim)",
              })}
            >
              <span style={{ fontSize: 15 }}>{icon}</span>
              {label}
            </NavLink>
          ))}
        </div>

        {/* User bar */}
        <div style={{
          padding: "12px 16px",
          borderTop: "1px solid var(--border)",
          display: "flex", alignItems: "center", justifyContent: "space-between",
        }}>
          <div style={{ fontSize: 13, color: "var(--fg-dim)", overflow: "hidden" }}>
            <div style={{ fontWeight: 600, color: "var(--fg)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              {session?.username}
            </div>
            <div style={{ fontSize: 11, color: "var(--muted)" }}>Instance Admin</div>
          </div>
          <button
            onClick={logout}
            className="btn btn-ghost"
            style={{ padding: "6px 10px", fontSize: 12 }}
            title="Sign out"
          >↩</button>
        </div>
      </nav>

      {/* Main content */}
      <main style={{ flex: 1, overflow: "auto", padding: "28px 32px" }}>
        {children}
      </main>
    </div>
  );
}

// ── Auth guard ────────────────────────────────────────────────────────────────

function RequireAuth({ children }: { children: React.ReactNode }) {
  const session = useSession();
  if (!session) return null; // App resolves identity before rendering routes.
  return <Shell>{children}</Shell>;
}

const AUTH_LOGIN_URL =
  (import.meta as unknown as { env?: Record<string, string | undefined> }).env
    ?.VITE_AUTH_LOGIN_URL ?? "https://auth.tnhc.dev/login";

function Centered({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-screen flex-col items-center justify-center gap-4 text-center">
      {children}
    </div>
  );
}

// ── App ───────────────────────────────────────────────────────────────────────

export default function App() {
  // Three outcomes, not two. "Signed in but not an instance admin" is a
  // different thing from "not signed in", and telling someone to sign in when
  // they already are is a dead end they cannot escape.
  const [state, setState] = useState<"checking" | "admin" | "forbidden" | "anonymous">(
    "checking",
  );

  useEffect(() => {
    let cancelled = false;
    void whoami().then((r) => {
      if (cancelled) return;
      if (r.state === "admin") setSession(r.session);
      setState(r.state);
    });
    return () => { cancelled = true; };
  }, []);

  if (state === "checking") return <Centered>Checking your access…</Centered>;

  if (state === "anonymous") {
    return (
      <Centered>
        <p>Not signed in.</p>
        <a
          className="rounded bg-blue-600 px-4 py-2 text-white"
          href={`${AUTH_LOGIN_URL}?redirect_uri=${encodeURIComponent(window.location.href)}`}
        >
          Sign in
        </a>
      </Centered>
    );
  }

  if (state === "forbidden") {
    return (
      <Centered>
        <p>This account does not have Instance Admin privileges.</p>
        <p className="text-sm opacity-70">
          Ask an operator to grant the INSTANCE_ADMIN flag, then reload.
        </p>
      </Centered>
    );
  }

  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<Navigate to="/overview" replace />} />
        <Route path="/overview"   element={<RequireAuth><OverviewPage /></RequireAuth>} />
        <Route path="/users"      element={<RequireAuth><UsersPage /></RequireAuth>} />
        <Route path="/users/:id"  element={<RequireAuth><UserDetailPage /></RequireAuth>} />
        <Route path="/servers"    element={<RequireAuth><ServersPage /></RequireAuth>} />
        <Route path="/moderation" element={<RequireAuth><ModerationPage /></RequireAuth>} />
        <Route path="/federation" element={<RequireAuth><FederationPage /></RequireAuth>} />
        <Route path="/audit"      element={<RequireAuth><AuditLogPage /></RequireAuth>} />
        <Route path="*"           element={<Navigate to="/overview" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

import { useEffect, useState } from "react";
import { api, type InstanceOverview } from "../api";
import NetworkHealth from "../components/NetworkHealth";

function formatUptime(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function StatCard({ label, value, sub, color }: {
  label: string; value: string | number; sub?: string; color?: string;
}) {
  return (
    <div className="stat-card">
      <div className="stat-label">{label}</div>
      <div className="stat-value" style={{ color: color ?? "var(--fg)" }}>{value}</div>
      {sub && <div className="stat-sub">{sub}</div>}
    </div>
  );
}

export default function OverviewPage() {
  const [data, setData] = useState<InstanceOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const d = await api.overview();
        if (!cancelled) { setData(d); setLoading(false); }
      } catch (e) {
        if (!cancelled) { setError((e as Error).message); setLoading(false); }
      }
    };
    load();
    const interval = setInterval(load, 30_000);
    return () => { cancelled = true; clearInterval(interval); };
  }, []);

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Overview</h1>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span className={`status-dot ${data?.gateway_online ? "status-online" : "status-offline"}`} />
          <span style={{ fontSize: 13, color: "var(--muted)" }}>
            {data?.gateway_online ? "Gateway online" : "Gateway offline"}
          </span>
        </div>
      </div>

      {error && (
        <div style={{
          background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.3)",
          borderRadius: 10, padding: "12px 16px", color: "#f87171", marginBottom: 20,
        }}>{error}</div>
      )}

      {loading && !data ? (
        <p style={{ color: "var(--muted)" }}>Loading…</p>
      ) : data ? (
        <>
          <div className="stat-grid" style={{ marginBottom: 24 }}>
            <StatCard label="Total users" value={data.total_users.toLocaleString()} />
            <StatCard
              label="Active (24h)"
              value={data.active_users_24h.toLocaleString()}
              sub={`${Math.round((data.active_users_24h / Math.max(data.total_users, 1)) * 100)}% of users`}
              color="var(--success)"
            />
            <StatCard
              label="Suspended"
              value={data.suspended_users.toLocaleString()}
              color={data.suspended_users > 0 ? "var(--warning)" : undefined}
            />
            <StatCard label="Servers" value={data.total_servers.toLocaleString()} />
            <StatCard label="Messages" value={data.total_messages.toLocaleString()} />
            <StatCard
              label="Voice connections"
              value={data.voice_connections}
              color={data.voice_connections > 0 ? "var(--accent)" : undefined}
            />
            <StatCard label="Uptime" value={formatUptime(data.uptime_secs)} />
            <StatCard label="Version" value={`v${data.version}`} />
          </div>

          {/* System status */}
          <div className="card">
            <h2 style={{ fontSize: 14, fontWeight: 700, marginBottom: 16, color: "var(--fg)" }}>
              System status
            </h2>
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {[
                { label: "REST API",        ok: true },
                { label: "WebSocket gateway", ok: data.gateway_online },
                { label: "Database",          ok: true },
              ].map(({ label, ok }) => (
                <div key={label} style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                  <span style={{ color: "var(--fg-dim)", fontSize: 14 }}>{label}</span>
                  <span style={{
                    fontSize: 12, fontWeight: 600, padding: "3px 10px", borderRadius: 20,
                    background: ok ? "rgba(74,222,128,0.12)" : "rgba(239,68,68,0.12)",
                    color: ok ? "var(--success)" : "var(--danger)",
                  }}>
                    {ok ? "Operational" : "Degraded"}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </>
      ) : null}
      <NetworkHealth />
    </div>
  );
}

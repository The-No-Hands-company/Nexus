import { useEffect, useState } from "react";

const API_URL = (import.meta as any).env?.VITE_NEXUS_NETWORK_API
  ?? "https://network.nexus.computer/api/stats";

interface NetworkStats {
  nodes_online: number;
  nodes_delta_hour: number;
  total_ram_gb: number;
  ram_used_pct: number;
  total_storage_gb: number;
  total_cpu_cores: number;
  avg_cpu_load_pct: number;
  compute_jobs_hour: number;
  countries: number;
  product_counts: Record<string, number>;
}

function fmtRam(gb: number) {
  if (gb >= 1024) return (gb / 1024).toFixed(1) + " TB";
  return Math.round(gb) + " GB";
}
function fmtNum(n: number) {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1000) return (n / 1000).toFixed(1) + "K";
  return n.toLocaleString();
}

export default function NetworkHealth() {
  const [stats, setStats] = useState<NetworkStats | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const load = () =>
      fetch(API_URL)
        .then(r => r.json())
        .then(d => { if (!cancelled) { setStats(d); setError(false); } })
        .catch(() => { if (!cancelled) setError(true); });

    load();
    const t = setInterval(load, 30_000);
    return () => { cancelled = true; clearInterval(t); };
  }, []);

  if (error) return null;

  return (
    <div style={{
      background: "var(--panel)",
      border: "1px solid var(--border)",
      borderRadius: 10,
      padding: "16px 18px",
      marginTop: 20,
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 14 }}>
        <span style={{
          width: 7, height: 7, borderRadius: "50%",
          background: stats ? "var(--live)" : "var(--muted)",
          display: "inline-block",
          animation: stats ? "pulse 2s infinite" : "none",
        }} />
        <span style={{ fontSize: 13, fontWeight: 600, color: "var(--fg)" }}>
          Nexus Network
        </span>
        <span style={{ fontSize: 11, color: "var(--muted)", marginLeft: "auto" }}>
          {stats ? "live" : "connecting…"}
        </span>
      </div>

      {!stats ? (
        <div style={{ height: 60, display: "flex", alignItems: "center", justifyContent: "center" }}>
          <div className="spinner" />
        </div>
      ) : (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 8, marginBottom: 14 }}>
            {[
              { label: "Nodes", value: fmtNum(stats.nodes_online), color: "var(--live)" },
              { label: "RAM", value: fmtRam(stats.total_ram_gb), color: "var(--fg)" },
              { label: "CPU cores", value: fmtNum(stats.total_cpu_cores), color: "var(--fg)" },
              { label: "Storage", value: fmtRam(stats.total_storage_gb), color: "var(--fg)" },
              { label: "Compute jobs", value: fmtNum(stats.compute_jobs_hour), color: "var(--warn)" },
              { label: "Countries", value: String(stats.countries), color: "var(--fg)" },
            ].map(m => (
              <div key={m.label} className="stat-card" style={{ padding: "8px 10px" }}>
                <div className="stat-label">{m.label}</div>
                <div className="stat-value" style={{ fontSize: 16, color: m.color }}>{m.value}</div>
              </div>
            ))}
          </div>

          <div style={{ fontSize: 11, color: "var(--muted)", textAlign: "center", paddingTop: 8, borderTop: "1px solid var(--border)" }}>
            <a href="https://github.com/The-No-Hands-company/Nexus-Network"
              target="_blank" rel="noopener noreferrer"
              style={{ color: "var(--muted)", textDecoration: "none" }}>
              Nexus Network · open source · voluntary telemetry
            </a>
          </div>
        </>
      )}
    </div>
  );
}

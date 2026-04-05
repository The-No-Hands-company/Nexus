import { useEffect, useState } from "react";
import { api, type FederationPeer } from "../api";
import { formatDistanceToNow } from "date-fns";

export default function FederationPage() {
  const [peers, setPeers] = useState<FederationPeer[]>([]);
  const [status, setStatus] = useState<Record<string, unknown> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const [st, ps] = await Promise.all([
        api.federationStatus(),
        api.federationPeers(),
      ]);
      setStatus(st);
      setPeers(ps.peers ?? []);
      setError(null);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  async function toggleBlock(domain: string, blocked: boolean) {
    setWorking(domain);
    try {
      if (blocked) await api.unblockPeer(domain);
      else await api.blockPeer(domain);
      await load();
    } catch (e) {
      alert((e as Error).message);
    } finally {
      setWorking(null);
    }
  }

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Federation</h1>
        <button className="btn btn-ghost" onClick={load}>Refresh</button>
      </div>

      {error && <div style={{ color: "var(--danger)", marginBottom: 12 }}>{error}</div>}

      {/* Status card */}
      {status && (
        <div className="card" style={{ marginBottom: 16 }}>
          <h2 style={{ fontSize: 14, fontWeight: 700, marginBottom: 12, color: "var(--fg)" }}>
            Federation status
          </h2>
          <div style={{ display: "flex", gap: 24, flexWrap: "wrap" }}>
            {Object.entries(status).slice(0, 6).map(([k, v]) => (
              <div key={k}>
                <div style={{ fontSize: 11, color: "var(--muted)", textTransform: "uppercase", letterSpacing: "0.8px" }}>{k.replace(/_/g, " ")}</div>
                <div style={{ fontSize: 14, color: "var(--fg-dim)", marginTop: 2 }}>
                  {typeof v === "boolean" ? (v ? "Yes" : "No") : String(v)}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Peers table */}
      <div className="card" style={{ padding: 0, overflow: "hidden" }}>
        {loading && !peers.length ? (
          <p style={{ padding: 24, color: "var(--muted)" }}>Loading…</p>
        ) : !peers.length ? (
          <div className="empty-state">
            <h3>No federation peers</h3>
            <p>No other Nexus or Matrix servers have been configured.</p>
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Domain</th><th>Trust level</th><th>Status</th><th>Last seen</th><th>Added</th><th></th>
              </tr>
            </thead>
            <tbody>
              {peers.map(p => (
                <tr key={p.domain}>
                  <td><span style={{ fontWeight: 600, color: "var(--fg)", fontFamily: "monospace" }}>{p.domain}</span></td>
                  <td>
                    <span style={{
                      fontSize: 12, padding: "2px 8px", borderRadius: 20,
                      background: p.trust_level === "full" ? "rgba(74,222,128,0.1)" : "rgba(245,158,11,0.1)",
                      color: p.trust_level === "full" ? "var(--success)" : "var(--warning)",
                    }}>
                      {p.trust_level}
                    </span>
                  </td>
                  <td>
                    <span style={{
                      fontSize: 12, padding: "2px 8px", borderRadius: 20,
                      background: p.blocked ? "rgba(239,68,68,0.1)" : "rgba(74,222,128,0.1)",
                      color: p.blocked ? "var(--danger)" : "var(--success)",
                    }}>
                      {p.blocked ? "Blocked" : "Active"}
                    </span>
                  </td>
                  <td style={{ fontSize: 12, color: "var(--muted)" }}>
                    {p.last_seen_at
                      ? formatDistanceToNow(new Date(p.last_seen_at), { addSuffix: true })
                      : "Never"}
                  </td>
                  <td style={{ fontSize: 12, color: "var(--muted)" }}>
                    {formatDistanceToNow(new Date(p.added_at), { addSuffix: true })}
                  </td>
                  <td>
                    <button
                      className={`btn ${p.blocked ? "btn-ghost" : "btn-danger"}`}
                      onClick={() => toggleBlock(p.domain, p.blocked)}
                      disabled={working === p.domain}
                      style={{ padding: "5px 12px", fontSize: 12 }}
                    >
                      {p.blocked ? "Unblock" : "Block"}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

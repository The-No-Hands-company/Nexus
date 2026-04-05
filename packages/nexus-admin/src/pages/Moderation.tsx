import { useEffect, useState } from "react";
import { api, type AdminServer } from "../api";

export default function ModerationPage() {
  const [servers, setServers] = useState<AdminServer[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<string | null>(null);
  const [reports, setReports] = useState<unknown[]>([]);
  const [reportsLoading, setReportsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.servers()
      .then(d => { setServers(d.servers); setLoading(false); })
      .catch(e => { setError((e as Error).message); setLoading(false); });
  }, []);

  async function loadReports(serverId: string) {
    setSelected(serverId);
    setReportsLoading(true);
    try {
      const session = (await import("../api")).getSession();
      if (!session) return;
      const base = session.serverUrl;
      const res = await fetch(
        `${base}/api/v1/servers/${serverId}/reports?status=pending`,
        { headers: { Authorization: `Bearer ${session.accessToken}` } }
      );
      if (res.ok) setReports(await res.json());
    } catch {
      setReports([]);
    } finally {
      setReportsLoading(false);
    }
  }

  const selectedServer = servers.find(s => s.id === selected);

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Moderation</h1>
      </div>

      {error && <div style={{ color: "var(--danger)", marginBottom: 12 }}>{error}</div>}

      <div style={{ display: "grid", gridTemplateColumns: "260px 1fr", gap: 16 }}>
        {/* Server list */}
        <div className="card" style={{ padding: 0, height: "fit-content" }}>
          <div style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)", fontWeight: 700, fontSize: 13, color: "var(--fg)" }}>
            Servers
          </div>
          {loading ? (
            <p style={{ padding: 16, color: "var(--muted)" }}>Loading…</p>
          ) : servers.map(s => (
            <button
              key={s.id}
              onClick={() => loadReports(s.id)}
              style={{
                display: "block", width: "100%", textAlign: "left",
                padding: "10px 16px", background: selected === s.id ? "var(--accent-dim)" : "transparent",
                color: selected === s.id ? "var(--accent)" : "var(--fg-dim)",
                border: "none", borderBottom: "1px solid var(--border)",
                cursor: "pointer", fontSize: 13, fontWeight: selected === s.id ? 600 : 400,
              }}
            >
              {s.name}
              <span style={{ float: "right", fontSize: 11, color: "var(--muted)" }}>
                {s.member_count.toLocaleString()}
              </span>
            </button>
          ))}
        </div>

        {/* Reports panel */}
        <div>
          {!selected ? (
            <div className="card empty-state">
              <h3>Select a server</h3>
              <p>Choose a server to view its pending moderation reports.</p>
            </div>
          ) : (
            <div>
              <div style={{ marginBottom: 14, fontWeight: 700, fontSize: 15, color: "var(--fg)" }}>
                {selectedServer?.name} — Pending reports
              </div>
              <div className="card" style={{ padding: 0, overflow: "hidden" }}>
                {reportsLoading ? (
                  <p style={{ padding: 24, color: "var(--muted)" }}>Loading…</p>
                ) : !reports.length ? (
                  <div className="empty-state">
                    <h3>No pending reports</h3>
                    <p>This server has no unresolved moderation reports.</p>
                  </div>
                ) : (
                  <table>
                    <thead>
                      <tr>
                        <th>Report ID</th><th>Reason</th><th>Status</th><th>Created</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(reports as Record<string, string>[]).map((r) => (
                        <tr key={r.id}>
                          <td><code style={{ fontSize: 11 }}>{r.id?.slice(0, 8)}…</code></td>
                          <td style={{ color: "var(--fg-dim)" }}>{r.reason ?? "—"}</td>
                          <td>
                            <span style={{
                              fontSize: 11, padding: "2px 8px", borderRadius: 20,
                              background: r.status === "pending" ? "rgba(245,158,11,0.15)" : "rgba(74,222,128,0.1)",
                              color: r.status === "pending" ? "var(--warning)" : "var(--success)",
                            }}>
                              {r.status}
                            </span>
                          </td>
                          <td style={{ fontSize: 12, color: "var(--muted)" }}>
                            {r.created_at ? new Date(r.created_at).toLocaleDateString() : "—"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

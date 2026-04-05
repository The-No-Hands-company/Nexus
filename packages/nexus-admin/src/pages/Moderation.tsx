import { useEffect, useState } from "react";
import { api, type AdminServer, type ModerationReport } from "../api";
import { formatDistanceToNow } from "date-fns";

type FilterStatus = "pending" | "resolved" | "dismissed";

export default function ModerationPage() {
  const [servers, setServers]               = useState<AdminServer[]>([]);
  const [loadingServers, setLoadingServers] = useState(true);
  const [selected, setSelected]             = useState<string | null>(null);
  const [reports, setReports]               = useState<ModerationReport[]>([]);
  const [reportsLoading, setReportsLoading] = useState(false);
  const [filter, setFilter]                 = useState<FilterStatus>("pending");
  const [working, setWorking]               = useState<string | null>(null);
  const [error, setError]                   = useState<string | null>(null);

  useEffect(() => {
    api.servers()
      .then(d => { setServers(d.servers); setLoadingServers(false); })
      .catch(e => { setError((e as Error).message); setLoadingServers(false); });
  }, []);

  async function loadReports(serverId: string) {
    setSelected(serverId);
    setReportsLoading(true);
    try {
      const data = await api.serverReports(serverId);
      setReports(data.reports ?? []);
    } catch { setReports([]); }
    finally   { setReportsLoading(false); }
  }

  async function handleResolve(reportId: string) {
    if (!selected) return;
    setWorking(reportId);
    try {
      await api.resolveReport(selected, reportId);
      setReports(r => r.map(x => x.id === reportId ? { ...x, status: "resolved" } : x));
    } catch (e) { setError((e as Error).message); }
    finally   { setWorking(null); }
  }

  async function handleDismiss(reportId: string) {
    if (!selected) return;
    setWorking(reportId);
    try {
      await api.dismissReport(selected, reportId);
      setReports(r => r.map(x => x.id === reportId ? { ...x, status: "dismissed" } : x));
    } catch (e) { setError((e as Error).message); }
    finally   { setWorking(null); }
  }

  const selectedServer = servers.find(s => s.id === selected);
  const filtered       = reports.filter(r => r.status === filter);

  const statusColor = (s: string) =>
    s === "pending" ? "var(--warning)" : s === "resolved" ? "var(--success)" : "var(--muted)";

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Moderation</h1>
      </div>

      {error && (
        <div style={{
          background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.3)",
          borderRadius: 10, padding: "10px 14px", marginBottom: 14, color: "#f87171",
        }}>{error}</div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "240px 1fr", gap: 16, alignItems: "start" }}>
        {/* Server sidebar */}
        <div className="card" style={{ padding: 0, overflow: "hidden" }}>
          <div style={{ padding: "10px 14px", borderBottom: "1px solid var(--border)", fontWeight: 700, fontSize: 13, color: "var(--fg)" }}>
            Servers
          </div>
          {loadingServers ? (
            <p style={{ padding: 16, color: "var(--muted)", fontSize: 13 }}>Loading…</p>
          ) : servers.map(s => (
            <button key={s.id} onClick={() => loadReports(s.id)} style={{
              display: "flex", width: "100%", textAlign: "left", alignItems: "center",
              padding: "9px 14px", gap: 8,
              background: selected === s.id ? "var(--accent-dim)" : "transparent",
              color: selected === s.id ? "var(--accent)" : "var(--fg-dim)",
              border: "none", borderBottom: "1px solid var(--border)",
              cursor: "pointer", fontSize: 13, fontWeight: selected === s.id ? 600 : 400,
            }}>
              <span style={{ flex: 1 }}>{s.name}</span>
              <span style={{ fontSize: 11, color: "var(--muted)" }}>
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
              <p>Choose a server to review its moderation reports.</p>
            </div>
          ) : (
            <>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
                <h2 style={{ fontSize: 15, fontWeight: 700, color: "var(--fg)", flex: 1 }}>
                  {selectedServer?.name} — Reports
                </h2>
                {(["pending", "resolved", "dismissed"] as FilterStatus[]).map(f => (
                  <button key={f}
                    className={`btn ${filter === f ? "btn-primary" : "btn-ghost"}`}
                    style={{ fontSize: 11, padding: "4px 12px" }}
                    onClick={() => setFilter(f)}
                  >
                    {f} ({reports.filter(r => r.status === f).length})
                  </button>
                ))}
                <button className="btn btn-ghost"
                  style={{ fontSize: 11, padding: "4px 10px" }}
                  onClick={() => loadReports(selected)}>↻</button>
              </div>

              <div className="card" style={{ padding: 0, overflow: "hidden" }}>
                {reportsLoading ? (
                  <p style={{ padding: 24, color: "var(--muted)", textAlign: "center" }}>Loading…</p>
                ) : !filtered.length ? (
                  <div className="empty-state">
                    <h3>No {filter} reports</h3>
                    <p>{filter === "pending" ? "All reports reviewed." : `No ${filter} reports.`}</p>
                  </div>
                ) : (
                  <table>
                    <thead>
                      <tr>
                        <th>ID</th><th>Reporter</th><th>Reported</th>
                        <th>Reason</th><th>Age</th><th>Status</th>
                        {filter === "pending" && <th>Actions</th>}
                      </tr>
                    </thead>
                    <tbody>
                      {filtered.map(r => (
                        <tr key={r.id}>
                          <td><code style={{ fontSize: 11, color: "var(--muted)" }}>{r.id.slice(0, 8)}…</code></td>
                          <td><code style={{ fontSize: 11 }}>{r.reporter_id.slice(0, 8)}…</code></td>
                          <td>
                            {r.reported_user_id
                              ? <code style={{ fontSize: 11 }}>{r.reported_user_id.slice(0, 8)}…</code>
                              : <span style={{ color: "var(--muted)" }}>—</span>}
                          </td>
                          <td style={{ maxWidth: 200 }}>
                            <span style={{ fontSize: 13, color: "var(--fg-dim)" }}>{r.reason || "—"}</span>
                          </td>
                          <td style={{ fontSize: 12, color: "var(--muted)", whiteSpace: "nowrap" }}>
                            {formatDistanceToNow(new Date(r.created_at), { addSuffix: true })}
                          </td>
                          <td>
                            <span style={{
                              fontSize: 11, padding: "2px 8px", borderRadius: 20,
                              background: `${statusColor(r.status)}22`,
                              color: statusColor(r.status), fontWeight: 600,
                            }}>{r.status}</span>
                          </td>
                          {filter === "pending" && (
                            <td>
                              <div style={{ display: "flex", gap: 4 }}>
                                <button className="btn btn-ghost"
                                  style={{ fontSize: 11, padding: "3px 10px" }}
                                  disabled={working === r.id}
                                  onClick={() => handleResolve(r.id)}>✓ Resolve</button>
                                <button className="btn btn-danger"
                                  style={{ fontSize: 11, padding: "3px 10px" }}
                                  disabled={working === r.id}
                                  onClick={() => handleDismiss(r.id)}>✗ Dismiss</button>
                              </div>
                            </td>
                          )}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

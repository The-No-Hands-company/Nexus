import { useEffect, useState } from "react";
import { api, type AuditEntry } from "../api";
import { format } from "date-fns";

export default function AuditLogPage() {
  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try {
      const data = await api.federationAudit();
      setEntries(data.entries ?? []);
      setError(null);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const actionColor = (action: string) => {
    if (action.includes("block"))   return "var(--danger)";
    if (action.includes("remove"))  return "var(--warning)";
    if (action.includes("add") || action.includes("accept")) return "var(--success)";
    return "var(--fg-dim)";
  };

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Audit Log</h1>
        <button className="btn btn-ghost" onClick={load}>Refresh</button>
      </div>

      {error && <div style={{ color: "var(--danger)", marginBottom: 12 }}>{error}</div>}

      <div className="card" style={{ padding: 0, overflow: "hidden" }}>
        {loading ? (
          <p style={{ padding: 24, color: "var(--muted)" }}>Loading…</p>
        ) : !entries.length ? (
          <div className="empty-state">
            <h3>No audit entries</h3>
            <p>Federation admin actions will appear here.</p>
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Time</th><th>Action</th><th>Target</th><th>Admin</th><th>Details</th>
              </tr>
            </thead>
            <tbody>
              {entries.map(e => (
                <tr key={e.id}>
                  <td style={{ fontSize: 12, color: "var(--muted)", whiteSpace: "nowrap" }}>
                    {format(new Date(e.created_at), "MMM d, HH:mm:ss")}
                  </td>
                  <td>
                    <span style={{ fontWeight: 600, fontSize: 13, color: actionColor(e.action) }}>
                      {e.action}
                    </span>
                  </td>
                  <td>
                    <code style={{ fontSize: 12, color: "var(--fg-dim)" }}>{e.target_domain}</code>
                  </td>
                  <td>
                    <code style={{ fontSize: 11, color: "var(--muted)" }}>{e.admin_id.slice(0, 8)}…</code>
                  </td>
                  <td style={{ fontSize: 12, color: "var(--muted)" }}>
                    {Object.keys(e.details ?? {}).length > 0
                      ? JSON.stringify(e.details).slice(0, 60)
                      : "—"}
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

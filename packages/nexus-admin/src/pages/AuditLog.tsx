import { useEffect, useState } from "react";
import { api, type AuditEntry } from "../api";
import { formatDistanceToNow, format } from "date-fns";

type FilterScope = "federation" | "all";

const ACTION_COLORS: Record<string, string> = {
  block:    "badge-suspended",
  unblock:  "badge-default",
  remove:   "badge-suspended",
  add:      "badge-default",
  accept:   "badge-default",
  reject:   "badge-suspended",
  update:   "badge-default",
  peer:     "badge-default",
};

function actionClass(action: string): string {
  for (const [key, cls] of Object.entries(ACTION_COLORS)) {
    if (action.toLowerCase().includes(key)) return cls;
  }
  return "badge-default";
}

function DetailCell({ details }: { details: Record<string, unknown> }) {
  const keys = Object.keys(details ?? {});
  if (!keys.length) return <span style={{ color: "var(--muted)" }}>—</span>;
  return (
    <details style={{ cursor: "pointer" }}>
      <summary style={{ color: "var(--muted)", fontSize: 12 }}>
        {keys.length} field{keys.length !== 1 ? "s" : ""}
      </summary>
      <pre style={{
        fontSize: 11, color: "var(--fg-dim)", marginTop: 4,
        background: "var(--bg-600)", borderRadius: 6, padding: "6px 8px",
        maxWidth: 320, overflow: "auto",
      }}>
        {JSON.stringify(details, null, 2)}
      </pre>
    </details>
  );
}

export default function AuditLogPage() {
  const [entries, setEntries]   = useState<AuditEntry[]>([]);
  const [loading, setLoading]   = useState(true);
  const [error, setError]       = useState<string | null>(null);
  const [search, setSearch]     = useState("");
  const [scope]                 = useState<FilterScope>("federation");
  const [autoRefresh, setAutoRefresh] = useState(false);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const data = await api.federationAudit();
      setEntries(data.entries ?? []);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { load(); }, []);

  useEffect(() => {
    if (!autoRefresh) return;
    const id = setInterval(load, 15_000);
    return () => clearInterval(id);
  }, [autoRefresh]);

  const filtered = entries.filter((e) => {
    if (!search) return true;
    const q = search.toLowerCase();
    return (
      e.action.toLowerCase().includes(q) ||
      e.target_domain.toLowerCase().includes(q) ||
      e.admin_id.toLowerCase().includes(q)
    );
  });

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Audit Log</h1>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, color: "var(--muted)", cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
            />
            Auto-refresh
          </label>
          <button className="btn btn-ghost" onClick={load} disabled={loading}>
            {loading ? "Loading…" : "↻ Refresh"}
          </button>
        </div>
      </div>

      {/* Stats row */}
      <div className="stat-grid" style={{ marginBottom: 16 }}>
        <div className="stat-card">
          <div className="stat-label">Total entries</div>
          <div className="stat-value">{entries.length}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Filtered</div>
          <div className="stat-value">{filtered.length}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Last action</div>
          <div className="stat-value" style={{ fontSize: 14 }}>
            {entries[0]
              ? formatDistanceToNow(new Date(entries[0].created_at), { addSuffix: true })
              : "—"}
          </div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Scope</div>
          <div className="stat-value" style={{ fontSize: 14, textTransform: "capitalize" }}>
            {scope}
          </div>
        </div>
      </div>

      {/* Search */}
      <div className="search-bar">
        <input
          placeholder="Search action, domain, or admin ID…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      {error && (
        <div style={{
          background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.3)",
          borderRadius: 10, padding: "12px 16px", marginBottom: 16, color: "#f87171",
        }}>
          {error}
        </div>
      )}

      <div className="card" style={{ padding: 0, overflow: "hidden" }}>
        {loading && !entries.length ? (
          <p style={{ padding: 32, color: "var(--muted)", textAlign: "center" }}>Loading…</p>
        ) : !filtered.length ? (
          <div className="empty-state">
            <h3>{search ? "No matching entries" : "No audit entries"}</h3>
            <p>{search ? "Try a different search term." : "Admin actions on federation peers will appear here."}</p>
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th style={{ width: 140 }}>Time</th>
                <th>Action</th>
                <th>Target domain</th>
                <th>Admin</th>
                <th>Details</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((e) => (
                <tr key={e.id}>
                  <td title={format(new Date(e.created_at), "yyyy-MM-dd HH:mm:ss")}>
                    <div style={{ fontSize: 12, color: "var(--muted)", whiteSpace: "nowrap" }}>
                      {format(new Date(e.created_at), "MMM d, HH:mm:ss")}
                    </div>
                    <div style={{ fontSize: 11, color: "var(--muted)", opacity: 0.6 }}>
                      {formatDistanceToNow(new Date(e.created_at), { addSuffix: true })}
                    </div>
                  </td>
                  <td>
                    <span className={`badge ${actionClass(e.action)}`}>
                      {e.action.replace(/_/g, " ")}
                    </span>
                  </td>
                  <td>
                    <code style={{ fontSize: 13, color: "var(--fg)" }}>{e.target_domain || "—"}</code>
                  </td>
                  <td>
                    <span
                      title={e.admin_id}
                      style={{ fontSize: 12, fontFamily: "monospace", color: "var(--muted)" }}
                    >
                      {e.admin_id.slice(0, 8)}…
                    </span>
                  </td>
                  <td>
                    <DetailCell details={e.details ?? {}} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {filtered.length > 0 && (
        <p style={{ fontSize: 12, color: "var(--muted)", marginTop: 12, textAlign: "right" }}>
          Showing {filtered.length} of {entries.length} entries
        </p>
      )}
    </div>
  );
}

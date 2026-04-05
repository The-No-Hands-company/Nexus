import { useEffect, useState } from "react";
import { api, type FederationAuditEntry, type InstanceAuditEntry } from "../api";
import { formatDistanceToNow, format } from "date-fns";

type Tab = "instance" | "federation";

function badgeClass(action: string): string {
  const l = action.toLowerCase();
  if (l.includes("suspend") || l.includes("block") || l.includes("remove") ||
      l.includes("disable") || l.includes("reject") || l.includes("delete")) return "badge-suspended";
  if (l.includes("grant") || l.includes("accept") || l.includes("add") ||
      l.includes("enable") || l.includes("unsuspend")) return "badge-default";
  return "badge-default";
}

function DetailCell({ data }: { data: Record<string, unknown> }) {
  const keys = Object.keys(data ?? {});
  if (!keys.length) return <span style={{ color: "var(--muted)" }}>—</span>;
  return (
    <details style={{ cursor: "pointer" }}>
      <summary style={{ fontSize: 12, color: "var(--muted)" }}>
        {keys.length} field{keys.length !== 1 ? "s" : ""}
      </summary>
      <pre style={{
        fontSize: 11, color: "var(--fg-dim)", marginTop: 4,
        background: "var(--bg-600)", borderRadius: 6, padding: "6px 8px",
        maxWidth: 280, overflow: "auto",
      }}>
        {JSON.stringify(data, null, 2)}
      </pre>
    </details>
  );
}

export default function AuditLogPage() {
  const [tab, setTab]                           = useState<Tab>("instance");
  const [instanceEntries, setInstanceEntries]   = useState<InstanceAuditEntry[]>([]);
  const [fedEntries, setFedEntries]             = useState<FederationAuditEntry[]>([]);
  const [loading, setLoading]                   = useState(true);
  const [error, setError]                       = useState<string | null>(null);
  const [search, setSearch]                     = useState("");
  const [autoRefresh, setAutoRefresh]           = useState(false);

  async function load() {
    setLoading(true); setError(null);
    try {
      const [inst, fed] = await Promise.all([
        api.instanceAudit({ limit: 200 }),
        api.federationAudit(),
      ]);
      setInstanceEntries(inst.entries ?? []);
      setFedEntries(fed.entries ?? []);
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

  const q = search.toLowerCase();
  const filtInst = instanceEntries.filter(e =>
    !q || e.action.toLowerCase().includes(q) ||
    e.actor_id.toLowerCase().includes(q) ||
    (e.target_type ?? "").toLowerCase().includes(q)
  );
  const filtFed = fedEntries.filter(e =>
    !q || e.action.toLowerCase().includes(q) ||
    e.target_domain.toLowerCase().includes(q) ||
    e.admin_id.toLowerCase().includes(q)
  );

  const active = tab === "instance" ? filtInst : filtFed;
  const total  = tab === "instance" ? instanceEntries.length : fedEntries.length;

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Audit Log</h1>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 13, color: "var(--muted)", cursor: "pointer" }}>
            <input type="checkbox" checked={autoRefresh}
              onChange={e => setAutoRefresh(e.target.checked)} />
            Auto-refresh
          </label>
          <button className="btn btn-ghost" onClick={load} disabled={loading}>
            {loading ? "Loading…" : "↻ Refresh"}
          </button>
        </div>
      </div>

      {/* Tabs */}
      <div style={{ display: "flex", gap: 4, borderBottom: "1px solid var(--border)", marginBottom: 16 }}>
        {(["instance", "federation"] as Tab[]).map(t => (
          <button key={t}
            className={`btn ${tab === t ? "btn-primary" : "btn-ghost"}`}
            style={{ fontSize: 12, padding: "5px 14px", borderRadius: "8px 8px 0 0" }}
            onClick={() => setTab(t)}
          >
            {t === "instance"
              ? `Instance actions (${instanceEntries.length})`
              : `Federation (${fedEntries.length})`}
          </button>
        ))}
      </div>

      {/* Stats row */}
      <div className="stat-grid" style={{ marginBottom: 16 }}>
        <div className="stat-card">
          <div className="stat-label">Total</div>
          <div className="stat-value">{total}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Filtered</div>
          <div className="stat-value">{active.length}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Last action</div>
          <div className="stat-value" style={{ fontSize: 13 }}>
            {active[0]
              ? formatDistanceToNow(new Date(active[0].created_at), { addSuffix: true })
              : "—"}
          </div>
        </div>
      </div>

      <div className="search-bar" style={{ marginBottom: 12 }}>
        <input
          placeholder={tab === "instance" ? "Search action, actor ID…" : "Search action, domain…"}
          value={search}
          onChange={e => setSearch(e.target.value)}
        />
      </div>

      {error && (
        <div style={{
          background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.3)",
          borderRadius: 10, padding: "10px 14px", marginBottom: 12, color: "#f87171",
        }}>{error}</div>
      )}

      <div className="card" style={{ padding: 0, overflow: "hidden" }}>
        {loading && !instanceEntries.length ? (
          <p style={{ padding: 32, color: "var(--muted)", textAlign: "center" }}>Loading…</p>
        ) : !active.length ? (
          <div className="empty-state">
            <h3>{search ? "No matching entries" : "No entries yet"}</h3>
            <p>{search ? "Try a different search." : "Admin actions will appear here."}</p>
          </div>
        ) : tab === "instance" ? (
          <table>
            <thead>
              <tr>
                <th>Time</th><th>Action</th><th>Target</th><th>Actor</th><th>Changes</th>
              </tr>
            </thead>
            <tbody>
              {filtInst.map(e => (
                <tr key={e.id}>
                  <td title={format(new Date(e.created_at), "yyyy-MM-dd HH:mm:ss")}>
                    <div style={{ fontSize: 12, color: "var(--muted)", whiteSpace: "nowrap" }}>
                      {format(new Date(e.created_at), "MMM d, HH:mm")}
                    </div>
                    <div style={{ fontSize: 11, color: "var(--muted)", opacity: 0.6 }}>
                      {formatDistanceToNow(new Date(e.created_at), { addSuffix: true })}
                    </div>
                  </td>
                  <td>
                    <span className={`badge ${badgeClass(e.action)}`}>
                      {e.action.replace(/_/g, " ")}
                    </span>
                  </td>
                  <td>
                    <div style={{ fontSize: 12, color: "var(--muted)" }}>{e.target_type ?? "—"}</div>
                    {e.target_id && (
                      <code style={{ fontSize: 11, color: "var(--muted)", opacity: 0.7 }}>
                        {e.target_id.slice(0, 8)}…
                      </code>
                    )}
                  </td>
                  <td>
                    <span title={e.actor_id}
                      style={{ fontSize: 12, fontFamily: "monospace", color: "var(--muted)" }}>
                      {e.actor_id.slice(0, 8)}…
                    </span>
                  </td>
                  <td><DetailCell data={e.changes ?? {}} /></td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Time</th><th>Action</th><th>Domain</th><th>Admin</th><th>Details</th>
              </tr>
            </thead>
            <tbody>
              {filtFed.map(e => (
                <tr key={e.id}>
                  <td title={format(new Date(e.created_at), "yyyy-MM-dd HH:mm:ss")}>
                    <div style={{ fontSize: 12, color: "var(--muted)", whiteSpace: "nowrap" }}>
                      {format(new Date(e.created_at), "MMM d, HH:mm")}
                    </div>
                    <div style={{ fontSize: 11, color: "var(--muted)", opacity: 0.6 }}>
                      {formatDistanceToNow(new Date(e.created_at), { addSuffix: true })}
                    </div>
                  </td>
                  <td>
                    <span className={`badge ${badgeClass(e.action)}`}>
                      {e.action.replace(/_/g, " ")}
                    </span>
                  </td>
                  <td>
                    <code style={{ fontSize: 13, color: "var(--fg)" }}>
                      {e.target_domain || "—"}
                    </code>
                  </td>
                  <td>
                    <span title={e.admin_id}
                      style={{ fontSize: 12, fontFamily: "monospace", color: "var(--muted)" }}>
                      {e.admin_id.slice(0, 8)}…
                    </span>
                  </td>
                  <td><DetailCell data={e.details ?? {}} /></td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {active.length < total && (
        <p style={{ fontSize: 12, color: "var(--muted)", marginTop: 10, textAlign: "right" }}>
          Showing {active.length} of {total}
        </p>
      )}
    </div>
  );
}

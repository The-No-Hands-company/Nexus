// ── Servers page ──────────────────────────────────────────────────────────────

import { useEffect, useState } from "react";
import { api, type AdminServer } from "../api";
import { formatDistanceToNow } from "date-fns";

export function ServersPage() {
  const [data, setData] = useState<{ servers: AdminServer[]; total: number } | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [deleting, setDeleting] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    try { setData(await api.servers()); setError(null); }
    catch (e) { setError((e as Error).message); }
    finally { setLoading(false); }
  };

  useEffect(() => { load(); }, []);

  const filtered = data?.servers.filter(s =>
    !search || s.name.toLowerCase().includes(search.toLowerCase())
  ) ?? [];

  async function confirmDelete(s: AdminServer) {
    if (!confirm(`Delete server "${s.name}"? This removes all channels and messages. This CANNOT be undone.`)) return;
    setDeleting(s.id);
    try { await api.deleteServer(s.id); await load(); }
    catch (e) { alert((e as Error).message); }
    finally { setDeleting(null); }
  }

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Servers</h1>
        <span style={{ color: "var(--muted)", fontSize: 13 }}>
          {data ? `${data.total.toLocaleString()} total` : ""}
        </span>
      </div>
      <div className="search-bar">
        <input placeholder="Filter by name…" value={search} onChange={e => setSearch(e.target.value)} />
        <button className="btn btn-ghost" onClick={load}>Refresh</button>
      </div>
      {error && <div style={{ color: "var(--danger)", marginBottom: 12 }}>{error}</div>}
      <div className="card" style={{ padding: 0, overflow: "hidden" }}>
        {loading && !data ? <p style={{ padding: 24, color: "var(--muted)" }}>Loading…</p>
        : !filtered.length ? (
          <div className="empty-state"><h3>No servers</h3></div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Name</th><th>Members</th><th>Owner</th><th>Boost</th><th>Visibility</th><th>Created</th><th></th>
              </tr>
            </thead>
            <tbody>
              {filtered.map(s => (
                <tr key={s.id}>
                  <td>
                    <div style={{ fontWeight: 600, color: "var(--fg)" }}>{s.name}</div>
                    {s.description && <div style={{ fontSize: 12, color: "var(--muted)" }}>{s.description.slice(0, 60)}</div>}
                  </td>
                  <td style={{ color: "var(--fg-dim)" }}>{s.member_count.toLocaleString()}</td>
                  <td><code style={{ fontSize: 11, color: "var(--muted)" }}>{s.owner_id.slice(0, 8)}…</code></td>
                  <td style={{ color: s.boost_tier > 0 ? "var(--accent)" : "var(--muted)" }}>
                    {s.boost_tier > 0 ? `Tier ${s.boost_tier}` : "—"}
                  </td>
                  <td>
                    <span style={{
                      fontSize: 11, padding: "2px 8px", borderRadius: 20,
                      background: s.is_public ? "rgba(74,222,128,0.1)" : "rgba(255,255,255,0.06)",
                      color: s.is_public ? "var(--success)" : "var(--muted)",
                    }}>
                      {s.is_public ? "Public" : "Private"}
                    </span>
                  </td>
                  <td style={{ fontSize: 12, color: "var(--muted)" }}>
                    {formatDistanceToNow(new Date(s.created_at), { addSuffix: true })}
                  </td>
                  <td>
                    <button
                      className="btn btn-danger"
                      onClick={() => confirmDelete(s)}
                      disabled={deleting === s.id}
                      style={{ padding: "5px 12px", fontSize: 12 }}
                    >
                      {deleting === s.id ? "…" : "Delete"}
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

export default ServersPage;

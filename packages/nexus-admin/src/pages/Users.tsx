import { useEffect, useState, useCallback } from "react";
import { Link } from "react-router-dom";
import { api, USER_FLAGS, type AdminUser, type UserList } from "../api";
import { formatDistanceToNow } from "date-fns";

function FlagBadge({ label }: { label: string }) {
  const cls =
    label === "instance_admin" ? "badge-admin"
    : label === "suspended"    ? "badge-suspended"
    : label === "disabled"     ? "badge-disabled"
    : label === "bot"          ? "badge-bot"
    : label === "staff"        ? "badge-staff"
    : "badge-default";
  return <span className={`badge ${cls}`}>{label}</span>;
}

const FILTER_OPTIONS = [
  { label: "All users",        flag: undefined, not_flag: undefined },
  { label: "Admins",           flag: USER_FLAGS.INSTANCE_ADMIN, not_flag: undefined },
  { label: "Suspended",        flag: USER_FLAGS.SUSPENDED, not_flag: undefined },
  { label: "Disabled",         flag: USER_FLAGS.DISABLED, not_flag: undefined },
  { label: "Bots",             flag: USER_FLAGS.BOT, not_flag: undefined },
  { label: "Staff",            flag: USER_FLAGS.STAFF, not_flag: undefined },
];

export default function UsersPage() {
  const [data, setData] = useState<UserList | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [filterIdx, setFilterIdx] = useState(0);
  const [page, setPage] = useState(0);
  const pageSize = 50;

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const filter = FILTER_OPTIONS[filterIdx];
      const params: Record<string, string | number> = {
        limit: pageSize,
        offset: page * pageSize,
        ...(search ? { q: search } : {}),
        ...(filter.flag !== undefined ? { flag: filter.flag } : {}),
        ...(filter.not_flag !== undefined ? { not_flag: filter.not_flag } : {}),
      };
      setData(await api.users(params));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }, [search, filterIdx, page]);

  useEffect(() => { load(); }, [load]);

  // Reset to page 0 when filter/search changes
  useEffect(() => { setPage(0); }, [search, filterIdx]);

  const totalPages = data ? Math.ceil(data.total / pageSize) : 0;

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Users</h1>
        <span style={{ color: "var(--muted)", fontSize: 13 }}>
          {data ? `${data.total.toLocaleString()} total` : ""}
        </span>
      </div>

      {/* Filters */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16, flexWrap: "wrap" }}>
        {FILTER_OPTIONS.map((opt, i) => (
          <button
            key={i}
            className={`btn ${i === filterIdx ? "btn-primary" : "btn-ghost"}`}
            onClick={() => setFilterIdx(i)}
            style={{ padding: "6px 14px", fontSize: 13 }}
          >
            {opt.label}
          </button>
        ))}
      </div>

      {/* Search */}
      <div className="search-bar">
        <input
          placeholder="Search by username or display name…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <button className="btn btn-ghost" onClick={load}>Refresh</button>
      </div>

      {error && (
        <div style={{
          background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.3)",
          borderRadius: 8, padding: "10px 14px", color: "#f87171", marginBottom: 14,
        }}>{error}</div>
      )}

      <div className="card" style={{ padding: 0, overflow: "hidden" }}>
        {loading && !data ? (
          <p style={{ padding: 24, color: "var(--muted)" }}>Loading…</p>
        ) : !data?.users.length ? (
          <div className="empty-state">
            <h3>No users found</h3>
            <p>Try adjusting the filter or search query.</p>
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>User</th>
                <th>Email</th>
                <th>Flags</th>
                <th>Presence</th>
                <th>Joined</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {data.users.map((u) => (
                <UserRow key={u.id} user={u} onRefresh={load} />
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div style={{ display: "flex", gap: 8, marginTop: 16, alignItems: "center", justifyContent: "flex-end" }}>
          <button className="btn btn-ghost" onClick={() => setPage(p => Math.max(0, p - 1))} disabled={page === 0}>
            ← Prev
          </button>
          <span style={{ color: "var(--muted)", fontSize: 13 }}>
            Page {page + 1} of {totalPages}
          </span>
          <button className="btn btn-ghost" onClick={() => setPage(p => Math.min(totalPages - 1, p + 1))} disabled={page >= totalPages - 1}>
            Next →
          </button>
        </div>
      )}
    </div>
  );
}

function UserRow({ user: u, onRefresh }: { user: AdminUser; onRefresh: () => void }) {
  const [working, setWorking] = useState(false);

  async function toggleSuspend() {
    setWorking(true);
    try {
      const isSuspended = u.flags & USER_FLAGS.SUSPENDED;
      if (isSuspended) {
        await api.updateUser(u.id, { clear_flags: USER_FLAGS.SUSPENDED });
      } else {
        await api.updateUser(u.id, { set_flags: USER_FLAGS.SUSPENDED });
      }
      onRefresh();
    } catch (e) {
      alert((e as Error).message);
    } finally {
      setWorking(false);
    }
  }

  const isSuspended = !!(u.flags & USER_FLAGS.SUSPENDED);

  return (
    <tr>
      <td>
        <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
          <Link to={`/users/${u.id}`} style={{ fontWeight: 600, color: "var(--fg)" }}>
            {u.username}
          </Link>
          {u.display_name && (
            <span style={{ fontSize: 12, color: "var(--muted)" }}>{u.display_name}</span>
          )}
        </div>
      </td>
      <td style={{ fontSize: 13, color: "var(--muted)" }}>
        {u.email ?? <span style={{ color: "var(--bg-500)" }}>—</span>}
      </td>
      <td>
        <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
          {u.flag_labels.map(l => <FlagBadge key={l} label={l} />)}
          {u.flag_labels.length === 0 && (
            <span style={{ color: "var(--bg-500)", fontSize: 13 }}>—</span>
          )}
        </div>
      </td>
      <td>
        <span className={`status-dot status-${u.presence === "online" ? "online" : u.presence === "idle" ? "idle" : "offline"}`} />
        <span style={{ fontSize: 13, color: "var(--muted)" }}>{u.presence}</span>
      </td>
      <td style={{ fontSize: 13, color: "var(--muted)" }}>
        {formatDistanceToNow(new Date(u.created_at), { addSuffix: true })}
      </td>
      <td>
        <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
          <button
            className={`btn ${isSuspended ? "btn-ghost" : "btn-danger"}`}
            onClick={toggleSuspend}
            disabled={working}
            style={{ padding: "5px 12px", fontSize: 12 }}
          >
            {isSuspended ? "Unsuspend" : "Suspend"}
          </button>
          <Link to={`/users/${u.id}`}>
            <button className="btn btn-ghost" style={{ padding: "5px 12px", fontSize: 12 }}>
              Details
            </button>
          </Link>
        </div>
      </td>
    </tr>
  );
}

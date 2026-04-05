import { useEffect, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { api, USER_FLAGS, type AdminUser } from "../api";
import { format } from "date-fns";

const FLAG_DEFS = [
  { flag: USER_FLAGS.INSTANCE_ADMIN,   label: "Instance Admin",    desc: "Full access to the admin panel and all admin API routes." },
  { flag: USER_FLAGS.STAFF,            label: "Staff",             desc: "Nexus staff member." },
  { flag: USER_FLAGS.SUSPENDED,        label: "Suspended",         desc: "Cannot sign in. Sessions invalidated." },
  { flag: USER_FLAGS.DISABLED,         label: "Disabled",          desc: "Account disabled — soft delete." },
  { flag: USER_FLAGS.BOT,              label: "Bot",               desc: "Automated user account." },
  { flag: USER_FLAGS.EMAIL_VERIFIED,   label: "Email verified",    desc: "Email address has been confirmed." },
  { flag: USER_FLAGS.EARLY_SUPPORTER,  label: "Early supporter",   desc: "Joined during early access period." },
  { flag: USER_FLAGS.PREMIUM_SUPPORTER,label: "Premium supporter", desc: "Active premium subscription." },
];

export default function UserDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [user, setUser] = useState<AdminUser | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const load = async () => {
    if (!id) return;
    setLoading(true);
    try {
      setUser(await api.getUser(id));
      setError(null);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, [id]);

  async function toggleFlag(flag: number, current: boolean) {
    if (!id) return;
    setWorking(true);
    setMessage(null);
    try {
      if (current) {
        await api.updateUser(id, { clear_flags: flag });
      } else {
        await api.updateUser(id, { set_flags: flag });
      }
      await load();
      setMessage("User updated.");
    } catch (e) {
      setMessage(`Error: ${(e as Error).message}`);
    } finally {
      setWorking(false);
    }
  }

  if (loading) return <p style={{ color: "var(--muted)" }}>Loading…</p>;
  if (error)   return <p style={{ color: "var(--danger)" }}>{error}</p>;
  if (!user)   return null;

  return (
    <div>
      <div style={{ marginBottom: 20 }}>
        <Link to="/users" style={{ color: "var(--muted)", fontSize: 13 }}>← Users</Link>
      </div>

      <div className="page-header">
        <div>
          <h1 className="page-title">{user.username}</h1>
          {user.display_name && (
            <p style={{ color: "var(--muted)", marginTop: 2 }}>{user.display_name}</p>
          )}
        </div>
        <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
          {user.flag_labels.map(l => (
            <span
              key={l}
              className={`badge ${
                l === "instance_admin" ? "badge-admin"
                : l === "suspended" ? "badge-suspended"
                : l === "disabled" ? "badge-disabled"
                : l === "bot" ? "badge-bot"
                : l === "staff" ? "badge-staff"
                : "badge-default"
              }`}
            >{l}</span>
          ))}
        </div>
      </div>

      {message && (
        <div style={{
          background: message.startsWith("Error") ? "rgba(239,68,68,0.1)" : "rgba(74,222,128,0.1)",
          border: `1px solid ${message.startsWith("Error") ? "rgba(239,68,68,0.3)" : "rgba(74,222,128,0.3)"}`,
          borderRadius: 8, padding: "10px 14px",
          color: message.startsWith("Error") ? "#f87171" : "#4ade80",
          marginBottom: 16, fontSize: 13,
        }}>{message}</div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
        {/* Account info */}
        <div className="card">
          <h2 style={{ fontSize: 14, fontWeight: 700, marginBottom: 14, color: "var(--fg)" }}>Account</h2>
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {[
              { label: "User ID",  value: user.id },
              { label: "Email",    value: user.email ?? "—" },
              { label: "Presence", value: user.presence },
              { label: "Remote",   value: user.is_remote ? "Yes (federated)" : "Local" },
              { label: "Joined",   value: format(new Date(user.created_at), "PPP") },
              { label: "Flags (raw)", value: `0x${user.flags.toString(16).toUpperCase()} (${user.flags})` },
            ].map(({ label, value }) => (
              <div key={label} style={{ display: "flex", justifyContent: "space-between", gap: 10 }}>
                <span style={{ color: "var(--muted)", fontSize: 13 }}>{label}</span>
                <span style={{ color: "var(--fg-dim)", fontSize: 13, fontFamily: label === "User ID" ? "monospace" : undefined }}>
                  {value}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Flag toggles */}
        <div className="card">
          <h2 style={{ fontSize: 14, fontWeight: 700, marginBottom: 14, color: "var(--fg)" }}>Flags</h2>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {FLAG_DEFS.map(({ flag, label, desc }) => {
              const active = !!(user.flags & flag);
              const isDangerous = flag === USER_FLAGS.SUSPENDED || flag === USER_FLAGS.DISABLED;
              return (
                <div key={flag} style={{
                  display: "flex", alignItems: "center", justifyContent: "space-between",
                  padding: "8px 10px", borderRadius: 8,
                  background: active ? (isDangerous ? "rgba(239,68,68,0.07)" : "rgba(39,201,165,0.07)") : "transparent",
                }}>
                  <div>
                    <div style={{ fontSize: 13, fontWeight: 600, color: active ? "var(--fg)" : "var(--fg-dim)" }}>{label}</div>
                    <div style={{ fontSize: 11, color: "var(--muted)" }}>{desc}</div>
                  </div>
                  <button
                    className={`btn ${active ? (isDangerous ? "btn-danger" : "btn-ghost") : "btn-ghost"}`}
                    onClick={() => toggleFlag(flag, active)}
                    disabled={working}
                    style={{ padding: "4px 12px", fontSize: 12, flexShrink: 0, marginLeft: 10 }}
                  >
                    {active ? "Remove" : "Set"}
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      </div>
      {/* Quick actions */}
      <div className="card" style={{ marginBottom: 16 }}>
        <h2 style={{ fontSize: 14, fontWeight: 700, marginBottom: 14, color: "var(--fg)" }}>Quick Actions</h2>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
          {!!(user.flags & USER_FLAGS.SUSPENDED) ? (
            <button className="btn btn-ghost" disabled={working} onClick={async () => {
              setWorking(true);
              try { await api.unsuspendUser(user.id); await load(); setMessage("Unsuspended."); }
              catch (e) { setMessage(`Error: ${(e as Error).message}`); }
              finally { setWorking(false); }
            }}>✓ Unsuspend</button>
          ) : (
            <button className="btn btn-danger" disabled={working} onClick={async () => {
              if (!confirm(`Suspend @${user.username}? They will be signed out.`)) return;
              setWorking(true);
              try { await api.suspendUser(user.id); await load(); setMessage("Suspended."); }
              catch (e) { setMessage(`Error: ${(e as Error).message}`); }
              finally { setWorking(false); }
            }}>⊘ Suspend</button>
          )}
          {!!(user.flags & USER_FLAGS.DISABLED) ? (
            <button className="btn btn-ghost" disabled={working} onClick={async () => {
              setWorking(true);
              try { await api.updateUser(user.id, { clear_flags: USER_FLAGS.DISABLED }); await load(); setMessage("Re-enabled."); }
              catch (e) { setMessage(`Error: ${(e as Error).message}`); }
              finally { setWorking(false); }
            }}>✓ Re-enable</button>
          ) : (
            <button className="btn btn-danger" disabled={working} onClick={async () => {
              if (!confirm(`Disable @${user.username}? Soft-delete — cannot sign in.`)) return;
              setWorking(true);
              try { await api.disableUser(user.id); await load(); setMessage("Disabled."); }
              catch (e) { setMessage(`Error: ${(e as Error).message}`); }
              finally { setWorking(false); }
            }}>✗ Disable</button>
          )}
          {!(user.flags & USER_FLAGS.INSTANCE_ADMIN) ? (
            <button className="btn btn-ghost" disabled={working} onClick={async () => {
              if (!confirm(`Grant full admin to @${user.username}?`)) return;
              setWorking(true);
              try { await api.updateUser(user.id, { set_flags: USER_FLAGS.INSTANCE_ADMIN }); await load(); setMessage("Admin granted."); }
              catch (e) { setMessage(`Error: ${(e as Error).message}`); }
              finally { setWorking(false); }
            }}>⬆ Grant Admin</button>
          ) : (
            <button className="btn btn-danger" disabled={working} onClick={async () => {
              if (!confirm(`Revoke admin from @${user.username}?`)) return;
              setWorking(true);
              try { await api.updateUser(user.id, { clear_flags: USER_FLAGS.INSTANCE_ADMIN }); await load(); setMessage("Admin revoked."); }
              catch (e) { setMessage(`Error: ${(e as Error).message}`); }
              finally { setWorking(false); }
            }}>⬇ Revoke Admin</button>
          )}
        </div>
      </div>
    </div>
  );
}

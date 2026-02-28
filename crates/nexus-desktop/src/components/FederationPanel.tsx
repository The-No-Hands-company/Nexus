/**
 * FederationPanel — instance-admin federation management panel.
 *
 * Rendered as a section at the bottom of the Settings page, visible to all
 * users but enforced server-side (returns 403 for non-admins). The panel
 * gracefully shows an "Instance admin access required" notice for regular users.
 *
 * Tabs:
 *   Status   — live federation health overview
 *   Identity — edit /.well-known display fields
 *   Peers    — manage federated servers (trust, block, health-check, remove)
 *   Requests — review and act on inbound/outbound peering invitations
 *   Audit    — paginated federation audit log
 *
 * v0.8.5 Federation UX
 */

import { useEffect, useState, useCallback } from "react";
import { invoke } from "../invoke";
import clsx from "clsx";

// ─── Types ────────────────────────────────────────────────────────────────────

interface FederationStatus {
  serverName: string;
  softwareVersion: string;
  federationEnabled: boolean;
  peerCount: number;
  healthyPeerCount: number;
  pendingInboundRequests: number;
  pendingOutboundRequests: number;
  uptimeSeconds: number;
}

type FederationPolicy = "open" | "closed" | "invite_only";

interface FederationIdentity {
  displayName?: string;
  description?: string;
  adminContact?: string;
  federationPolicy: FederationPolicy;
}

interface FederatedPeer {
  domain: string;
  displayName?: string;
  trustScore: number;
  isHealthy: boolean;
  isBlocked: boolean;
  latencyMs?: number;
  lastSeenAt?: string;
  notes?: string;
  addedAt: string;
}

interface PeeringRequest {
  id: string;
  direction: "inbound" | "outbound";
  remoteDomain: string;
  remoteDisplayName?: string;
  remoteDescription?: string;
  status: "pending" | "accepted" | "rejected" | "cancelled";
  message?: string;
  createdAt: string;
}

interface AuditEntry {
  id: string;
  adminId: string;
  action: string;
  targetDomain: string;
  details: Record<string, unknown>;
  createdAt: string;
}

interface PeerHealthResult {
  domain: string;
  status: "online" | "offline";
  latencyMs?: number;
  displayName?: string;
  softwareVersion?: string;
  userCount?: number;
  federationPolicy?: string;
  error?: string;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatRelative(iso?: string): string {
  if (!iso) return "—";
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

const TRUST_COLORS: Record<string, string> = {
  high:       "text-green-400",
  medium:     "text-yellow-400",
  low:        "text-orange-400",
  blocked:    "text-red-400",
};

function trustLabel(score: number, isBlocked: boolean): { label: string; cls: string } {
  if (isBlocked) return { label: "Blocked", cls: TRUST_COLORS.blocked };
  if (score >= 80)  return { label: `High (${score})`, cls: TRUST_COLORS.high };
  if (score >= 40)  return { label: `Medium (${score})`, cls: TRUST_COLORS.medium };
  return { label: `Low (${score})`, cls: TRUST_COLORS.low };
}

const ACTION_LABELS: Record<string, string> = {
  peer_added:         "Peer Added",
  peer_removed:       "Peer Removed",
  peer_blocked:       "Peer Blocked",
  peer_unblocked:     "Peer Unblocked",
  trust_updated:      "Trust Updated",
  request_accepted:   "Request Accepted",
  request_rejected:   "Request Rejected",
  identity_updated:   "Identity Updated",
};

function auditActionClass(action: string): string {
  if (action.includes("blocked"))   return "bg-red-500/20 text-red-300";
  if (action.includes("removed"))   return "bg-orange-500/20 text-orange-300";
  if (action.includes("accepted"))  return "bg-green-500/20 text-green-300";
  if (action.includes("rejected"))  return "bg-red-500/20 text-red-300";
  if (action.includes("added"))     return "bg-blue-500/20 text-blue-300";
  return "bg-bg-700 text-muted";
}

type Tab = "status" | "identity" | "peers" | "requests" | "audit";

const TABS: { id: Tab; label: string }[] = [
  { id: "status",   label: "Status" },
  { id: "identity", label: "Identity" },
  { id: "peers",    label: "Peers" },
  { id: "requests", label: "Requests" },
  { id: "audit",    label: "Audit Log" },
];

// ─── Component ────────────────────────────────────────────────────────────────

export default function FederationPanel() {
  const [activeTab, setActiveTab] = useState<Tab>("status");
  const [authError, setAuthError] = useState(false);
  const [globalError, setGlobalError] = useState<string | null>(null);

  // ── Status ────────────────────────────────────────────────────────────
  const [status, setStatus] = useState<FederationStatus | null>(null);
  const [statusLoading, setStatusLoading] = useState(false);

  const loadStatus = useCallback(async () => {
    setStatusLoading(true);
    setGlobalError(null);
    try {
      const data = await invoke<FederationStatus>("get_federation_status", {});
      setStatus(data);
    } catch (e: unknown) {
      const msg = (e as Error).message ?? String(e);
      if (msg.includes("403") || msg.toLowerCase().includes("forbidden")) {
        setAuthError(true);
      } else {
        setGlobalError(msg);
      }
    } finally {
      setStatusLoading(false);
    }
  }, []);

  useEffect(() => { loadStatus(); }, [loadStatus]);

  // ── Identity ─────────────────────────────────────────────────────────
  const [identity, setIdentity] = useState<FederationIdentity | null>(null);
  const [identityLoading, setIdentityLoading] = useState(false);
  const [identitySaving, setIdentitySaving] = useState(false);
  const [identityError, setIdentityError] = useState<string | null>(null);
  const [identitySuccess, setIdentitySuccess] = useState(false);
  const [identityForm, setIdentityForm] = useState<FederationIdentity>({
    federationPolicy: "open",
  });

  const loadIdentity = useCallback(async () => {
    setIdentityLoading(true);
    try {
      const data = await invoke<FederationIdentity>("get_federation_identity", {});
      setIdentity(data);
      setIdentityForm(data);
    } catch {
      // handled above
    } finally {
      setIdentityLoading(false);
    }
  }, []);

  useEffect(() => {
    if (activeTab === "identity" && !identity) loadIdentity();
  }, [activeTab, identity, loadIdentity]);

  const saveIdentity = async () => {
    setIdentitySaving(true);
    setIdentityError(null);
    setIdentitySuccess(false);
    try {
      await invoke("update_federation_identity", { ...identityForm });
      setIdentitySuccess(true);
      setTimeout(() => setIdentitySuccess(false), 3000);
      await loadIdentity();
    } catch (e: unknown) {
      setIdentityError((e as Error).message ?? String(e));
    } finally {
      setIdentitySaving(false);
    }
  };

  // ── Peers ─────────────────────────────────────────────────────────────
  const [peers, setPeers] = useState<FederatedPeer[]>([]);
  const [peersLoading, setPeersLoading] = useState(false);
  const [peersError, setPeersError] = useState<string | null>(null);

  // Add-peer form
  const [addDomain, setAddDomain] = useState("");
  const [addMessage, setAddMessage] = useState("");
  const [addTrust, setAddTrust] = useState(50);
  const [addingPeer, setAddingPeer] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const [addSuccess, setAddSuccess] = useState<string | null>(null);

  // Inline health results
  const [healthResults, setHealthResults] = useState<Record<string, PeerHealthResult>>({});
  const [checkingHealth, setCheckingHealth] = useState<Record<string, boolean>>({});

  // Trust edit inline
  const [trustEditing, setTrustEditing] = useState<Record<string, number>>({});
  const [savingTrust, setSavingTrust] = useState<Record<string, boolean>>({});

  const loadPeers = useCallback(async () => {
    setPeersLoading(true);
    setPeersError(null);
    try {
      const data = await invoke<{ peers: FederatedPeer[] }>("list_peers", {});
      setPeers(data.peers ?? []);
    } catch (e: unknown) {
      setPeersError((e as Error).message ?? String(e));
    } finally {
      setPeersLoading(false);
    }
  }, []);

  useEffect(() => {
    if (activeTab === "peers") loadPeers();
  }, [activeTab, loadPeers]);

  const handleAddPeer = async (e: React.FormEvent) => {
    e.preventDefault();
    setAddingPeer(true);
    setAddError(null);
    setAddSuccess(null);
    try {
      const result = await invoke<{ domain: string; display_name?: string; latency_ms: number }>(
        "add_peer", { domain: addDomain, message: addMessage || undefined, trustScore: addTrust }
      );
      setAddSuccess(
        `Peering request sent to ${result.domain}` +
        (result.display_name ? ` (${result.display_name})` : "") +
        ` — latency ${result.latency_ms}ms`
      );
      setAddDomain("");
      setAddMessage("");
      setAddTrust(50);
      await loadPeers();
    } catch (e: unknown) {
      setAddError((e as Error).message ?? String(e));
    } finally {
      setAddingPeer(false);
    }
  };

  const handleCheckHealth = async (domain: string) => {
    setCheckingHealth((prev) => ({ ...prev, [domain]: true }));
    try {
      const result = await invoke<PeerHealthResult>("peer_health", { domain });
      setHealthResults((prev) => ({ ...prev, [domain]: result }));
      await loadPeers();
    } catch (e: unknown) {
      setHealthResults((prev) => ({
        ...prev,
        [domain]: { domain, status: "offline", error: (e as Error).message },
      }));
    } finally {
      setCheckingHealth((prev) => ({ ...prev, [domain]: false }));
    }
  };

  const handleUpdateTrust = async (domain: string) => {
    const score = trustEditing[domain];
    if (score === undefined) return;
    setSavingTrust((prev) => ({ ...prev, [domain]: true }));
    try {
      await invoke("update_peer_trust", { domain, trustScore: score });
      setTrustEditing((prev) => { const n = { ...prev }; delete n[domain]; return n; });
      await loadPeers();
    } catch {
      // leave editing state open, user can retry
    } finally {
      setSavingTrust((prev) => ({ ...prev, [domain]: false }));
    }
  };

  const handleBlockPeer = async (domain: string) => {
    try {
      await invoke("block_peer", { domain });
      await loadPeers();
    } catch { /* ignore */ }
  };

  const handleUnblockPeer = async (domain: string) => {
    try {
      await invoke("unblock_peer", { domain });
      await loadPeers();
    } catch { /* ignore */ }
  };

  const handleRemovePeer = async (domain: string) => {
    if (!confirm(`Remove ${domain} from federation? This cannot be undone.`)) return;
    try {
      await invoke("remove_peer", { domain });
      await loadPeers();
    } catch { /* ignore */ }
  };

  // ── Requests ──────────────────────────────────────────────────────────
  const [requests, setRequests] = useState<PeeringRequest[]>([]);
  const [requestsLoading, setRequestsLoading] = useState(false);
  const [requestFilter, setRequestFilter] = useState<"pending" | "all">("pending");
  const [actingRequest, setActingRequest] = useState<Record<string, boolean>>({});

  const loadRequests = useCallback(async () => {
    setRequestsLoading(true);
    try {
      const status = requestFilter === "pending" ? "pending" : undefined;
      const data = await invoke<{ requests: PeeringRequest[] }>(
        "list_peering_requests", { status }
      );
      setRequests(data.requests ?? []);
    } catch { /* ignore */ } finally {
      setRequestsLoading(false);
    }
  }, [requestFilter]);

  useEffect(() => {
    if (activeTab === "requests") loadRequests();
  }, [activeTab, loadRequests]);

  const handleAccept = async (id: string) => {
    setActingRequest((prev) => ({ ...prev, [id]: true }));
    try {
      await invoke("accept_peering_request", { requestId: id });
      await loadRequests();
    } catch { /* ignore */ } finally {
      setActingRequest((prev) => ({ ...prev, [id]: false }));
    }
  };

  const handleReject = async (id: string) => {
    setActingRequest((prev) => ({ ...prev, [id]: true }));
    try {
      await invoke("reject_peering_request", { requestId: id });
      await loadRequests();
    } catch { /* ignore */ } finally {
      setActingRequest((prev) => ({ ...prev, [id]: false }));
    }
  };

  // ── Audit ─────────────────────────────────────────────────────────────
  const [auditEntries, setAuditEntries] = useState<AuditEntry[]>([]);
  const [auditLoading, setAuditLoading] = useState(false);
  const [auditDomainFilter, setAuditDomainFilter] = useState("");
  const [auditTotal, setAuditTotal] = useState(0);

  const loadAudit = useCallback(async () => {
    setAuditLoading(true);
    try {
      const data = await invoke<{ entries: AuditEntry[]; total: number }>(
        "get_federation_audit",
        { domain: auditDomainFilter || undefined, limit: 50 }
      );
      setAuditEntries(data.entries ?? []);
      setAuditTotal(data.total ?? 0);
    } catch { /* ignore */ } finally {
      setAuditLoading(false);
    }
  }, [auditDomainFilter]);

  useEffect(() => {
    if (activeTab === "audit") loadAudit();
  }, [activeTab, loadAudit]);

  // ─── Auth guard ───────────────────────────────────────────────────────

  if (authError) {
    return (
      <div className="rounded-xl border border-bg-600/50 bg-bg-800/50 p-6 text-center">
        <p className="text-sm font-medium text-fg mb-1">Instance Admin Access Required</p>
        <p className="text-xs text-muted">
          Federation management is restricted to instance administrators.
          Contact your instance owner to request access.
        </p>
      </div>
    );
  }

  // ─── Render ───────────────────────────────────────────────────────────

  return (
    <div className="space-y-4">
      {/* Tab bar */}
      <div className="flex gap-1 rounded-lg bg-bg-800/60 p-1 border border-bg-600/30">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setActiveTab(t.id)}
            className={clsx(
              "flex-1 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
              activeTab === t.id
                ? "bg-accent-600 text-white shadow"
                : "text-muted hover:text-fg hover:bg-bg-700"
            )}
          >
            {t.label}
            {t.id === "requests" && requests.filter((r) => r.status === "pending" && r.direction === "inbound").length > 0 && (
              <span className="ml-1.5 inline-flex h-4 w-4 items-center justify-center rounded-full bg-red-500 text-[10px] text-white">
                {requests.filter((r) => r.status === "pending" && r.direction === "inbound").length}
              </span>
            )}
          </button>
        ))}
      </div>

      {globalError && (
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-2 text-sm text-red-300">
          {globalError}
        </div>
      )}

      {/* ── Status tab ──────────────────────────────────────────────── */}
      {activeTab === "status" && (
        <div className="space-y-4">
          {statusLoading && !status && (
            <div className="text-center text-sm text-muted py-8">Loading federation status…</div>
          )}
          {status && (
            <>
              {/* Overview cards */}
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
                <StatCard
                  label="Peers"
                  value={String(status.peerCount)}
                  sub={`${status.healthyPeerCount} healthy`}
                  ok={status.healthyPeerCount === status.peerCount}
                />
                <StatCard
                  label="Inbound Requests"
                  value={String(status.pendingInboundRequests)}
                  sub="pending"
                  ok={status.pendingInboundRequests === 0}
                />
                <StatCard
                  label="Uptime"
                  value={formatUptime(status.uptimeSeconds)}
                  sub="since last restart"
                  ok={true}
                />
                <StatCard
                  label="Federation"
                  value={status.federationEnabled ? "Enabled" : "Disabled"}
                  sub={status.softwareVersion}
                  ok={status.federationEnabled}
                />
              </div>

              {/* Instance detail */}
              <div className="rounded-xl border border-bg-600/30 bg-bg-800/50 p-4 space-y-2">
                <p className="text-xs font-semibold uppercase tracking-wider text-muted">
                  This Instance
                </p>
                <div className="grid grid-cols-2 gap-x-6 gap-y-1 text-sm">
                  <InfoRow label="Server name" value={status.serverName} mono />
                  <InfoRow label="Software"    value={status.softwareVersion} />
                  <InfoRow
                    label="Healthy peers"
                    value={`${status.healthyPeerCount} / ${status.peerCount}`}
                  />
                  <InfoRow
                    label="Pending outbound"
                    value={String(status.pendingOutboundRequests)}
                  />
                </div>
              </div>

              <button
                type="button"
                onClick={loadStatus}
                className="text-xs text-accent-400 hover:text-accent-300 transition-colors"
              >
                ↻ Refresh
              </button>
            </>
          )}
        </div>
      )}

      {/* ── Identity tab ────────────────────────────────────────────── */}
      {activeTab === "identity" && (
        <div className="space-y-4">
          {identityLoading && !identity && (
            <div className="text-center text-sm text-muted py-8">Loading identity…</div>
          )}
          {(identity || !identityLoading) && (
            <form
              onSubmit={(e) => { e.preventDefault(); saveIdentity(); }}
              className="space-y-4"
            >
              <div className="rounded-xl border border-bg-600/30 bg-bg-800/50 p-4 space-y-4">
                <p className="text-xs font-semibold uppercase tracking-wider text-muted">
                  Instance Identity
                </p>
                <p className="text-xs text-muted leading-relaxed">
                  These fields appear in your instance's{" "}
                  <code className="text-accent-400">/.well-known/nexus/server</code> endpoint
                  and are shown to admins on other instances when they search or peer with you.
                </p>

                {/* Display Name */}
                <FormField label="Display Name" description="Human-readable name for your instance">
                  <input
                    type="text"
                    maxLength={64}
                    placeholder="My Nexus Instance"
                    value={identityForm.displayName ?? ""}
                    onChange={(e) => setIdentityForm((f) => ({ ...f, displayName: e.target.value }))}
                    className="w-full rounded-lg bg-bg-700 border border-bg-600/50 px-3 py-2 text-sm text-fg placeholder-muted focus:outline-none focus:ring-2 focus:ring-accent-500"
                  />
                </FormField>

                {/* Description */}
                <FormField label="Description" description="Brief description shown to other instances">
                  <textarea
                    rows={3}
                    maxLength={512}
                    placeholder="A community server for…"
                    value={identityForm.description ?? ""}
                    onChange={(e) => setIdentityForm((f) => ({ ...f, description: e.target.value }))}
                    className="w-full rounded-lg bg-bg-700 border border-bg-600/50 px-3 py-2 text-sm text-fg placeholder-muted focus:outline-none focus:ring-2 focus:ring-accent-500 resize-none"
                  />
                </FormField>

                {/* Admin Contact */}
                <FormField label="Admin Contact" description="Email or URI for federation inquiries">
                  <input
                    type="text"
                    maxLength={256}
                    placeholder="admin@example.com"
                    value={identityForm.adminContact ?? ""}
                    onChange={(e) => setIdentityForm((f) => ({ ...f, adminContact: e.target.value }))}
                    className="w-full rounded-lg bg-bg-700 border border-bg-600/50 px-3 py-2 text-sm text-fg placeholder-muted focus:outline-none focus:ring-2 focus:ring-accent-500"
                  />
                </FormField>

                {/* Federation Policy */}
                <FormField
                  label="Federation Policy"
                  description="Controls which remote instances can federate with you"
                >
                  <div className="grid grid-cols-3 gap-2">
                    {(["open", "closed", "invite_only"] as FederationPolicy[]).map((pol) => (
                      <label
                        key={pol}
                        className={clsx(
                          "flex flex-col gap-0.5 rounded-lg border px-3 py-2 cursor-pointer transition-colors",
                          identityForm.federationPolicy === pol
                            ? "border-accent-500 bg-accent-500/10"
                            : "border-bg-600/50 bg-bg-700 hover:bg-bg-600/50"
                        )}
                      >
                        <input
                          type="radio"
                          name="federationPolicy"
                          value={pol}
                          checked={identityForm.federationPolicy === pol}
                          onChange={() => setIdentityForm((f) => ({ ...f, federationPolicy: pol }))}
                          className="sr-only"
                        />
                        <span className="text-xs font-medium capitalize text-fg">
                          {pol.replace("_", " ")}
                        </span>
                        <span className="text-[10px] text-muted leading-tight">
                          {pol === "open" && "All instances can peer"}
                          {pol === "closed" && "No new peering allowed"}
                          {pol === "invite_only" && "Admin must approve peers"}
                        </span>
                      </label>
                    ))}
                  </div>
                </FormField>
              </div>

              {identityError && (
                <p className="text-xs text-red-400">{identityError}</p>
              )}
              {identitySuccess && (
                <p className="text-xs text-green-400">Identity saved.</p>
              )}

              <div className="flex justify-end">
                <button
                  type="submit"
                  disabled={identitySaving}
                  className="rounded-lg bg-accent-600 hover:bg-accent-500 disabled:opacity-50 px-4 py-2 text-sm font-medium text-white transition-colors"
                >
                  {identitySaving ? "Saving…" : "Save Identity"}
                </button>
              </div>
            </form>
          )}
        </div>
      )}

      {/* ── Peers tab ───────────────────────────────────────────────── */}
      {activeTab === "peers" && (
        <div className="space-y-4">
          {/* Add peer form */}
          <div className="rounded-xl border border-bg-600/30 bg-bg-800/50 p-4 space-y-3">
            <p className="text-xs font-semibold uppercase tracking-wider text-muted">
              Add Peer
            </p>
            <form onSubmit={handleAddPeer} className="space-y-3">
              <div className="grid gap-3 sm:grid-cols-3">
                <div className="sm:col-span-2">
                  <label className="block text-xs text-muted mb-1">Domain</label>
                  <input
                    type="text"
                    required
                    placeholder="nexus.other.example"
                    value={addDomain}
                    onChange={(e) => setAddDomain(e.target.value.trim())}
                    className="w-full rounded-lg bg-bg-700 border border-bg-600/50 px-3 py-2 text-sm text-fg placeholder-muted focus:outline-none focus:ring-2 focus:ring-accent-500"
                  />
                </div>
                <div>
                  <label className="block text-xs text-muted mb-1">
                    Initial Trust ({addTrust})
                  </label>
                  <input
                    type="range"
                    min={0}
                    max={100}
                    value={addTrust}
                    onChange={(e) => setAddTrust(Number(e.target.value))}
                    className="w-full accent-accent-500 h-2 mt-2"
                  />
                </div>
              </div>
              <div>
                <label className="block text-xs text-muted mb-1">
                  Message (optional, sent with peering request)
                </label>
                <input
                  type="text"
                  maxLength={256}
                  placeholder="Hey, let's federate!"
                  value={addMessage}
                  onChange={(e) => setAddMessage(e.target.value)}
                  className="w-full rounded-lg bg-bg-700 border border-bg-600/50 px-3 py-2 text-sm text-fg placeholder-muted focus:outline-none focus:ring-2 focus:ring-accent-500"
                />
              </div>
              {addError && <p className="text-xs text-red-400">{addError}</p>}
              {addSuccess && <p className="text-xs text-green-400">{addSuccess}</p>}
              <button
                type="submit"
                disabled={addingPeer || !addDomain}
                className="rounded-lg bg-accent-600 hover:bg-accent-500 disabled:opacity-50 px-4 py-2 text-sm font-medium text-white transition-colors"
              >
                {addingPeer ? "Contacting peer…" : "Send Peering Request"}
              </button>
            </form>
          </div>

          {/* Peers list */}
          {peersLoading && (
            <div className="text-center text-sm text-muted py-4">Loading peers…</div>
          )}
          {peersError && (
            <div className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-2 text-sm text-red-300">
              {peersError}
            </div>
          )}
          {!peersLoading && peers.length === 0 && !peersError && (
            <p className="text-center text-sm text-muted py-4">
              No federated peers yet. Add one above.
            </p>
          )}
          {peers.length > 0 && (
            <div className="rounded-xl border border-bg-600/30 overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-bg-600/30 bg-bg-800/80">
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted">Domain</th>
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted">Trust</th>
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted">Health</th>
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted">Last seen</th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-muted">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {peers.map((peer) => {
                    const trust = trustLabel(peer.trustScore, peer.isBlocked);
                    const hr = healthResults[peer.domain];
                    const editing = trustEditing[peer.domain] !== undefined;
                    return (
                      <tr key={peer.domain} className="border-b border-bg-600/20 hover:bg-bg-700/30 transition-colors">
                        <td className="px-4 py-2.5">
                          <p className="font-mono text-xs text-fg">{peer.domain}</p>
                          {peer.displayName && (
                            <p className="text-[11px] text-muted">{peer.displayName}</p>
                          )}
                        </td>
                        <td className="px-4 py-2.5">
                          {editing ? (
                            <div className="flex items-center gap-2">
                              <input
                                type="number"
                                min={0}
                                max={100}
                                value={trustEditing[peer.domain]}
                                onChange={(e) =>
                                  setTrustEditing((prev) => ({
                                    ...prev,
                                    [peer.domain]: Number(e.target.value),
                                  }))
                                }
                                className="w-16 rounded bg-bg-700 border border-bg-600/50 px-2 py-1 text-xs text-fg"
                              />
                              <button
                                type="button"
                                onClick={() => handleUpdateTrust(peer.domain)}
                                disabled={savingTrust[peer.domain]}
                                className="text-[11px] text-accent-400 hover:text-accent-300"
                              >
                                {savingTrust[peer.domain] ? "…" : "Save"}
                              </button>
                              <button
                                type="button"
                                onClick={() =>
                                  setTrustEditing((prev) => {
                                    const n = { ...prev };
                                    delete n[peer.domain];
                                    return n;
                                  })
                                }
                                className="text-[11px] text-muted hover:text-fg"
                              >
                                Cancel
                              </button>
                            </div>
                          ) : (
                            <button
                              type="button"
                              onClick={() =>
                                setTrustEditing((prev) => ({
                                  ...prev,
                                  [peer.domain]: peer.trustScore,
                                }))
                              }
                              className={clsx("text-xs hover:underline", trust.cls)}
                            >
                              {trust.label}
                            </button>
                          )}
                        </td>
                        <td className="px-4 py-2.5">
                          {hr ? (
                            <span
                              className={clsx(
                                "text-xs",
                                hr.status === "online" ? "text-green-400" : "text-red-400"
                              )}
                            >
                              {hr.status === "online"
                                ? `✓ ${hr.latencyMs}ms`
                                : `✗ ${hr.error ?? "offline"}`}
                            </span>
                          ) : (
                            <span
                              className={clsx(
                                "text-xs",
                                peer.isHealthy ? "text-green-400" : "text-red-400"
                              )}
                            >
                              {peer.isHealthy
                                ? `✓${peer.latencyMs !== undefined ? ` ${peer.latencyMs}ms` : ""}`
                                : "✗ offline"}
                            </span>
                          )}
                        </td>
                        <td className="px-4 py-2.5 text-xs text-muted">
                          {formatRelative(peer.lastSeenAt)}
                        </td>
                        <td className="px-4 py-2.5">
                          <div className="flex items-center justify-end gap-2">
                            <button
                              type="button"
                              onClick={() => handleCheckHealth(peer.domain)}
                              disabled={checkingHealth[peer.domain]}
                              className="text-[11px] text-accent-400 hover:text-accent-300 transition-colors disabled:opacity-50"
                              title="Check health"
                            >
                              {checkingHealth[peer.domain] ? "…" : "Ping"}
                            </button>
                            {peer.isBlocked ? (
                              <button
                                type="button"
                                onClick={() => handleUnblockPeer(peer.domain)}
                                className="text-[11px] text-green-400 hover:text-green-300 transition-colors"
                              >
                                Unblock
                              </button>
                            ) : (
                              <button
                                type="button"
                                onClick={() => handleBlockPeer(peer.domain)}
                                className="text-[11px] text-orange-400 hover:text-orange-300 transition-colors"
                              >
                                Block
                              </button>
                            )}
                            <button
                              type="button"
                              onClick={() => handleRemovePeer(peer.domain)}
                              className="text-[11px] text-red-400 hover:text-red-300 transition-colors"
                              title="Remove peer"
                            >
                              Remove
                            </button>
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {/* ── Requests tab ────────────────────────────────────────────── */}
      {activeTab === "requests" && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex gap-2">
              {(["pending", "all"] as const).map((f) => (
                <button
                  key={f}
                  type="button"
                  onClick={() => setRequestFilter(f)}
                  className={clsx(
                    "rounded-lg px-3 py-1 text-xs font-medium transition-colors",
                    requestFilter === f
                      ? "bg-accent-600 text-white"
                      : "bg-bg-700 text-muted hover:text-fg"
                  )}
                >
                  {f === "pending" ? "Pending" : "All"}
                </button>
              ))}
            </div>
            <button
              type="button"
              onClick={loadRequests}
              className="text-xs text-accent-400 hover:text-accent-300 transition-colors"
            >
              ↻ Refresh
            </button>
          </div>

          {requestsLoading && (
            <div className="text-center text-sm text-muted py-4">Loading requests…</div>
          )}

          {!requestsLoading && requests.length === 0 && (
            <p className="text-center text-sm text-muted py-6">
              No {requestFilter === "pending" ? "pending " : ""}peering requests.
            </p>
          )}

          {requests.length > 0 && (
            <div className="space-y-2">
              {requests.map((req) => (
                <div
                  key={req.id}
                  className="rounded-xl border border-bg-600/30 bg-bg-800/50 p-4 space-y-2"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="space-y-0.5">
                      <div className="flex items-center gap-2">
                        <span
                          className={clsx(
                            "rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase",
                            req.direction === "inbound"
                              ? "bg-blue-500/20 text-blue-300"
                              : "bg-purple-500/20 text-purple-300"
                          )}
                        >
                          {req.direction}
                        </span>
                        <span className="font-mono text-sm text-fg">{req.remoteDomain}</span>
                        {req.remoteDisplayName && (
                          <span className="text-xs text-muted">— {req.remoteDisplayName}</span>
                        )}
                        <StatusBadge status={req.status} />
                      </div>
                      {req.remoteDescription && (
                        <p className="text-xs text-muted">{req.remoteDescription}</p>
                      )}
                      {req.message && (
                        <p className="text-xs text-fg/80 italic">"{req.message}"</p>
                      )}
                    </div>
                    <p className="shrink-0 text-[11px] text-muted">{formatRelative(req.createdAt)}</p>
                  </div>

                  {req.direction === "inbound" && req.status === "pending" && (
                    <div className="flex gap-2 pt-1">
                      <button
                        type="button"
                        onClick={() => handleAccept(req.id)}
                        disabled={actingRequest[req.id]}
                        className="rounded-lg bg-green-600 hover:bg-green-500 disabled:opacity-50 px-3 py-1.5 text-xs font-medium text-white transition-colors"
                      >
                        {actingRequest[req.id] ? "…" : "Accept"}
                      </button>
                      <button
                        type="button"
                        onClick={() => handleReject(req.id)}
                        disabled={actingRequest[req.id]}
                        className="rounded-lg bg-bg-700 hover:bg-red-600/20 disabled:opacity-50 px-3 py-1.5 text-xs font-medium text-muted hover:text-red-300 transition-colors"
                      >
                        Reject
                      </button>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── Audit log tab ───────────────────────────────────────────── */}
      {activeTab === "audit" && (
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            <div className="relative flex-1">
              <input
                type="text"
                placeholder="Filter by domain…"
                value={auditDomainFilter}
                onChange={(e) => setAuditDomainFilter(e.target.value)}
                className="w-full rounded-lg bg-bg-700 border border-bg-600/50 px-3 py-2 text-sm text-fg placeholder-muted focus:outline-none focus:ring-2 focus:ring-accent-500"
              />
            </div>
            <button
              type="button"
              onClick={loadAudit}
              className="text-xs text-accent-400 hover:text-accent-300 transition-colors"
            >
              ↻ Refresh
            </button>
          </div>

          {auditTotal > 0 && (
            <p className="text-xs text-muted">{auditTotal} total entries</p>
          )}

          {auditLoading && (
            <div className="text-center text-sm text-muted py-4">Loading audit log…</div>
          )}

          {!auditLoading && auditEntries.length === 0 && (
            <p className="text-center text-sm text-muted py-6">No audit entries.</p>
          )}

          {auditEntries.length > 0 && (
            <div className="rounded-xl border border-bg-600/30 overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-bg-600/30 bg-bg-800/80">
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted">Action</th>
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted">Domain</th>
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted">Details</th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-muted">When</th>
                  </tr>
                </thead>
                <tbody>
                  {auditEntries.map((entry) => (
                    <tr
                      key={entry.id}
                      className="border-b border-bg-600/20 hover:bg-bg-700/30 transition-colors"
                    >
                      <td className="px-4 py-2.5">
                        <span
                          className={clsx(
                            "rounded px-1.5 py-0.5 text-[10px] font-medium",
                            auditActionClass(entry.action)
                          )}
                        >
                          {ACTION_LABELS[entry.action] ?? entry.action}
                        </span>
                      </td>
                      <td className="px-4 py-2.5 font-mono text-xs text-fg">
                        {entry.targetDomain || "—"}
                      </td>
                      <td className="px-4 py-2.5 text-xs text-muted max-w-xs truncate">
                        {Object.entries(entry.details)
                          .map(([k, v]) => `${k}: ${JSON.stringify(v)}`)
                          .join(", ") || "—"}
                      </td>
                      <td className="px-4 py-2.5 text-right text-xs text-muted whitespace-nowrap">
                        {formatRelative(entry.createdAt)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ─── Sub-components ───────────────────────────────────────────────────────────

function StatCard({
  label,
  value,
  sub,
  ok,
}: {
  label: string;
  value: string;
  sub: string;
  ok: boolean;
}) {
  return (
    <div className="rounded-xl border border-bg-600/30 bg-bg-800/50 p-3 space-y-0.5">
      <p className="text-[10px] uppercase tracking-wider text-muted font-medium">{label}</p>
      <p className={clsx("text-xl font-semibold", ok ? "text-fg" : "text-orange-400")}>{value}</p>
      <p className="text-[11px] text-muted">{sub}</p>
    </div>
  );
}

function InfoRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex justify-between gap-4 py-0.5">
      <span className="text-muted shrink-0">{label}</span>
      <span className={clsx("text-right text-fg truncate", mono && "font-mono")}>{value}</span>
    </div>
  );
}

function FormField({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1">
      <label className="block text-xs font-medium text-fg">{label}</label>
      {description && <p className="text-[11px] text-muted">{description}</p>}
      {children}
    </div>
  );
}

function StatusBadge({ status }: { status: PeeringRequest["status"] }) {
  const cls: Record<PeeringRequest["status"], string> = {
    pending:  "bg-yellow-500/20 text-yellow-300",
    accepted: "bg-green-500/20 text-green-300",
    rejected: "bg-red-500/20 text-red-300",
    cancelled:"bg-bg-700 text-muted",
  };
  return (
    <span className={clsx("rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase", cls[status])}>
      {status}
    </span>
  );
}

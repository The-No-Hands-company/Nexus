/**
 * FriendsPanel — shows friends list, pending requests, and Add Friend dialog.
 *
 * Tabs:
 *   Online  — accepted friends currently online
 *   All     — all accepted friends
 *   Pending — incoming (needs action) + outgoing requests
 *
 * Add Friend button at the top opens an inline form.
 */
import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useStore, Relationship } from "../store";
import { invoke } from "../invoke";
import clsx from "clsx";

/** Extract a human-readable message from a raw API error string or JSON body. */
function parseApiError(e: unknown): string {
  const raw = String(e).replace(/^Error: /, "");
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (typeof parsed.message === "string") return parsed.message;
    if (typeof parsed.error === "string") return parsed.error;
  } catch {
    // not JSON — use as-is
  }
  return raw;
}

type Tab = "online" | "all" | "pending";

interface UserBrief {
  id: string;
  username: string;
  displayName?: string;
  avatar?: string;
}

export default function FriendsPanel() {
  const { relationships, loadRelationships, loadDmChannels, appendDmChannel } =
    useStore();
  const navigate = useNavigate();

  const [tab, setTab] = useState<Tab>("all");
  const [showAddFriend, setShowAddFriend] = useState(false);
  const [addUsername, setAddUsername] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [addSuccess, setAddSuccess] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  useEffect(() => {
    loadRelationships();
  }, [loadRelationships]);

  // ── Tab filtering ──────────────────────────────────────────────────────────

  const friends = relationships.filter((r) => r.status === "accepted");
  const pending = relationships.filter(
    (r) => r.status === "pending"
  );
  const incoming = pending.filter((r) => r.direction === "incoming");
  const outgoing = pending.filter((r) => r.direction === "outgoing");

  const displayed: Relationship[] =
    tab === "all"
      ? friends
      : tab === "pending"
      ? pending
      : friends; // "online" filter — server doesn't push presence here so same as all for now

  // ── Actions ────────────────────────────────────────────────────────────────

  const handleAddFriend = async () => {
    if (!addUsername.trim()) return;
    setAddError(null);
    setAddSuccess(null);
    try {
      await invoke("send_friend_request", { username: addUsername.trim() });
      setAddSuccess(`Friend request sent to ${addUsername.trim()}!`);
      setAddUsername("");
      loadRelationships();
    } catch (e) {
      setAddError(parseApiError(e));
    }
  };

  const handleAccept = async (userId: string) => {
    setActionLoading(userId + ":accept");
    try {
      await invoke("update_relationship", { userId, action: "accept" });
      loadRelationships();
    } catch (e) {
      console.error("accept error", e);
    } finally {
      setActionLoading(null);
    }
  };

  const handleDecline = async (userId: string) => {
    setActionLoading(userId + ":decline");
    try {
      await invoke("update_relationship", { userId, action: "deny" });
      loadRelationships();
    } catch (e) {
      console.error("decline error", e);
    } finally {
      setActionLoading(null);
    }
  };

  const handleRemove = async (userId: string) => {
    setActionLoading(userId + ":remove");
    try {
      await invoke("delete_relationship", { userId });
      loadRelationships();
    } catch (e) {
      console.error("remove error", e);
    } finally {
      setActionLoading(null);
    }
  };

  const openDm = async (userId: string) => {
    try {
      const dm = await invoke<{
        id: string;
        channelType: "dm" | "group_dm";
        name?: string;
        lastMessageId?: string;
        recipients: UserBrief[];
      }>("create_dm", { recipientId: userId });
      appendDmChannel(dm);
      loadDmChannels();
      navigate(`/channel/${dm.id}`);
    } catch (e) {
      console.error("openDm error", e);
    }
  };

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center gap-3 px-4 py-3 border-b border-bg-600/50">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" className="text-fg opacity-70 shrink-0">
          <path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z"/>
        </svg>
        <span className="font-semibold text-fg text-sm">Friends</span>

        {/* Tabs */}
        <div className="flex gap-1 ml-2">
          {(["all", "online", "pending"] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={clsx(
                "px-2 py-0.5 rounded text-xs capitalize transition-colors",
                tab === t
                  ? "bg-accent-500 text-white"
                  : "text-muted hover:bg-bg-700 hover:text-fg"
              )}
            >
              {t}
              {t === "pending" && incoming.length > 0 && (
                <span className="ml-1 text-xs bg-red-500 text-white rounded-full px-1">
                  {incoming.length}
                </span>
              )}
            </button>
          ))}
        </div>

        {/* Add Friend button */}
        <button
          onClick={() => setShowAddFriend((v) => !v)}
          className={clsx(
            "ml-auto px-3 py-1 rounded text-xs font-medium transition-colors",
            showAddFriend
              ? "bg-bg-700 text-muted"
              : "bg-accent-500/20 text-accent-400 border border-accent-500/40 hover:bg-accent-500/30"
          )}
        >
          Add Friend
        </button>
      </div>

      {/* Add friend form */}
      {showAddFriend && (
        <div className="px-4 py-3 bg-bg-800 border-b border-bg-600/50">
          <p className="text-xs text-muted mb-1">
            Enter a username to add a local user, or{" "}
            <span className="font-mono text-accent-400">username@server.tld</span>{" "}
            to add someone on another Nexus server.
          </p>
          <div className="flex gap-2">
            <input
              type="text"
              value={addUsername}
              onChange={(e) => setAddUsername(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAddFriend()}
              placeholder="username  or  username@server.tld"
              className="flex-1 bg-bg-700 text-fg text-sm rounded px-2 py-1 border border-bg-600 outline-none focus:border-accent-500 placeholder:text-muted"
            />
            <button
              onClick={handleAddFriend}
              className="px-3 py-1 bg-accent-500 text-white text-xs rounded hover:bg-accent-600 transition-colors"
            >
              Send
            </button>
          </div>
          {addError && <p className="text-xs text-red-400 mt-1">{addError}</p>}
          {addSuccess && <p className="text-xs text-green-400 mt-1">{addSuccess}</p>}
        </div>
      )}

      {/* Friend list */}
      <div className="flex-1 overflow-y-auto px-2 py-2">
        {tab === "pending" ? (
          <>
            {incoming.length > 0 && (
              <div>
                <p className="text-xs text-muted uppercase tracking-wider px-2 py-1">
                  Incoming — {incoming.length}
                </p>
                {incoming.map((r) => (
                  <RelationshipRow
                    key={r.id}
                    rel={r}
                    type="incoming"
                    loading={actionLoading}
                    onAccept={() => handleAccept(r.user.id)}
                    onDecline={() => handleDecline(r.user.id)}
                  />
                ))}
              </div>
            )}
            {outgoing.length > 0 && (
              <div className={incoming.length > 0 ? "mt-2" : ""}>
                <p className="text-xs text-muted uppercase tracking-wider px-2 py-1">
                  Outgoing — {outgoing.length}
                </p>
                {outgoing.map((r) => (
                  <RelationshipRow
                    key={r.id}
                    rel={r}
                    type="outgoing"
                    loading={actionLoading}
                    onCancel={() => handleRemove(r.user.id)}
                  />
                ))}
              </div>
            )}
            {incoming.length === 0 && outgoing.length === 0 && (
              <EmptyState message="No pending requests" />
            )}
          </>
        ) : displayed.length === 0 ? (
          <EmptyState message="No friends yet. Add someone!" />
        ) : (
          <>
            <p className="text-xs text-muted uppercase tracking-wider px-2 py-1">
              {tab === "all" ? "All friends" : "Online"} — {displayed.length}
            </p>
            {displayed.map((r) => (
              <RelationshipRow
                key={r.id}
                rel={r}
                type="friend"
                loading={actionLoading}
                onMessage={() => openDm(r.user.id)}
                onRemove={() => handleRemove(r.user.id)}
              />
            ))}
          </>
        )}
      </div>
    </div>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────────

interface RelationshipRowProps {
  rel: Relationship;
  type: "friend" | "incoming" | "outgoing";
  loading: string | null;
  onAccept?: () => void;
  onDecline?: () => void;
  onCancel?: () => void;
  onMessage?: () => void;
  onRemove?: () => void;
}

function RelationshipRow({
  rel,
  type,
  loading,
  onAccept,
  onDecline,
  onCancel,
  onMessage,
  onRemove,
}: RelationshipRowProps) {
  const { user } = rel;
  const displayName = user.displayName ?? user.username;
  const initials = displayName.slice(0, 2).toUpperCase();

  return (
    <div className="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-bg-700/60 group cursor-default">
      {/* Avatar */}
      <div className="w-8 h-8 rounded-full bg-bg-600 flex items-center justify-center text-xs font-semibold text-muted shrink-0 overflow-hidden">
        {user.avatar ? (
          <img src={user.avatar} alt={displayName} className="w-full h-full object-cover" />
        ) : (
          initials
        )}
      </div>

      {/* Name */}
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-fg truncate">{displayName}</p>
        <p className="text-xs text-muted truncate">
          {user.username !== displayName ? `@${user.username}` : (
            type === "incoming" ? "Incoming request" :
            type === "outgoing" ? "Outgoing request" :
            "Friend"
          )}
        </p>
      </div>

      {/* Actions — visible on hover */}
      <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
        {type === "friend" && (
          <>
            <ActionBtn title="Message" onClick={onMessage}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z"/>
              </svg>
            </ActionBtn>
            <ActionBtn title="Remove friend" variant="danger" onClick={onRemove}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
              </svg>
            </ActionBtn>
          </>
        )}
        {type === "incoming" && (
          <>
            <ActionBtn title="Accept" variant="success" onClick={onAccept}
              disabled={loading === rel.user.id + ":accept"}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
              </svg>
            </ActionBtn>
            <ActionBtn title="Decline" variant="danger" onClick={onDecline}
              disabled={loading === rel.user.id + ":decline"}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
              </svg>
            </ActionBtn>
          </>
        )}
        {type === "outgoing" && (
          <ActionBtn title="Cancel request" variant="danger" onClick={onCancel}
            disabled={loading === rel.user.id + ":remove"}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
            </svg>
          </ActionBtn>
        )}
      </div>
    </div>
  );
}

function ActionBtn({
  title,
  variant = "default",
  disabled = false,
  onClick,
  children,
}: {
  title: string;
  variant?: "default" | "success" | "danger";
  disabled?: boolean;
  onClick?: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      title={title}
      disabled={disabled}
      onClick={onClick}
      className={clsx(
        "w-6 h-6 rounded-full flex items-center justify-center transition-colors disabled:opacity-50",
        variant === "success" && "bg-green-500/20 text-green-400 hover:bg-green-500/40",
        variant === "danger" && "bg-red-500/20 text-red-400 hover:bg-red-500/40",
        variant === "default" && "bg-bg-600/50 text-muted hover:bg-bg-600 hover:text-fg"
      )}
    >
      {children}
    </button>
  );
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-muted text-sm">
      <svg width="40" height="40" viewBox="0 0 24 24" fill="currentColor" className="opacity-30 mb-2">
        <path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z"/>
      </svg>
      <p>{message}</p>
    </div>
  );
}

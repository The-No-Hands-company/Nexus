/**
 * ServerBrowserModal — Federated server and room discovery browser.
 *
 * Two tabs:
 *   Servers — list of known federated Nexus servers (from /api/v1/directory/servers).
 *             Clicking a server filters the Rooms tab to that server.
 *   Rooms   — public federated rooms (from /api/v1/directory/rooms + /search).
 *             Search bar, optional server filter chip, Join button.
 *
 * Join flow: POST /api/v1/directory/rooms/join → make_join → send_join (backend handles it).
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "../invoke";
import clsx from "clsx";

interface DirectoryServer {
  serverName: string;
  description?: string;
  iconUrl?: string;
  publicRoomCount: number;
  totalUsers: number;
}

interface DirectoryRoom {
  roomId: string;
  name?: string;
  topic?: string;
  memberCount: number;
  serverName: string;
  joinRule: string;
  tags: string[];
}

interface DirectoryServerList {
  servers: DirectoryServer[];
  totalCount: number;
}

interface DirectoryRoomList {
  rooms: DirectoryRoom[];
  totalCount: number;
}

interface Props {
  onClose: () => void;
}

type Tab = "servers" | "rooms";

export default function ServerBrowserModal({ onClose }: Props) {
  const [tab, setTab] = useState<Tab>("servers");

  // Server list state
  const [servers, setServers] = useState<DirectoryServer[]>([]);
  const [serversLoading, setServersLoading] = useState(true);
  const [serversError, setServersError] = useState<string | null>(null);

  // Room list state
  const [rooms, setRooms] = useState<DirectoryRoom[]>([]);
  const [roomsLoading, setRoomsLoading] = useState(false);
  const [roomsError, setRoomsError] = useState<string | null>(null);

  // Filters
  const [serverFilter, setServerFilter] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [pendingSearch, setPendingSearch] = useState("");

  // Join state
  const [joiningRoomId, setJoiningRoomId] = useState<string | null>(null);
  const [joinedRooms, setJoinedRooms] = useState<Set<string>>(new Set());
  const [joinError, setJoinError] = useState<string | null>(null);

  // ── Load servers ──────────────────────────────────────────────────────────

  useEffect(() => {
    setServersLoading(true);
    setServersError(null);
    invoke<DirectoryServerList>("directory_list_servers", { limit: 100 })
      .then((data) => setServers(data.servers))
      .catch((e) => setServersError(String(e)))
      .finally(() => setServersLoading(false));
  }, []);

  // ── Load rooms ────────────────────────────────────────────────────────────

  const loadRooms = useCallback(
    (query: string, server: string | null) => {
      setRoomsLoading(true);
      setRoomsError(null);
      const cmd = query.trim() ? "directory_search_rooms" : "directory_list_rooms";
      const args: Record<string, unknown> = { limit: 100 };
      if (query.trim()) args.query = query.trim();
      if (server) args.server = server;
      invoke<DirectoryRoomList>(cmd, args)
        .then((data) => setRooms(data.rooms))
        .catch((e) => setRoomsError(String(e)))
        .finally(() => setRoomsLoading(false));
    },
    []
  );

  // Load rooms whenever tab switches to "rooms", or server filter changes,
  // or search term commits (Enter key or debounce).
  useEffect(() => {
    if (tab === "rooms") {
      loadRooms(searchQuery, serverFilter);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab, serverFilter, searchQuery]);

  // ── Browse a specific server ───────────────────────────────────────────────

  const browseServer = (serverName: string) => {
    setServerFilter(serverName);
    setSearchQuery("");
    setPendingSearch("");
    setTab("rooms");
  };

  // ── Join a room ───────────────────────────────────────────────────────────

  const handleJoin = async (roomId: string) => {
    setJoiningRoomId(roomId);
    setJoinError(null);
    try {
      await invoke("directory_join_room", { roomId });
      setJoinedRooms((prev) => new Set(prev).add(roomId));
    } catch (e) {
      const raw = String(e).replace(/^Error: /, "");
      // Try to parse JSON error body
      try {
        const parsed = JSON.parse(raw) as Record<string, unknown>;
        setJoinError(
          typeof parsed.error === "string"
            ? parsed.error
            : typeof parsed.message === "string"
            ? parsed.message
            : raw
        );
      } catch {
        setJoinError(raw);
      }
    } finally {
      setJoiningRoomId(null);
    }
  };

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="bg-bg-800 rounded-xl shadow-2xl w-[720px] max-w-[calc(100vw-32px)] flex flex-col"
           style={{ height: "min(600px, calc(100vh - 64px))" }}>

        {/* ── Header ─────────────────────────────────────────────────────── */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-bg-600/50 shrink-0">
          <div className="flex items-center gap-3">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" className="text-accent-400">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
            </svg>
            <h2 className="text-base font-bold text-fg">Browse Servers</h2>
          </div>
          <button onClick={onClose} className="text-muted hover:text-fg transition-colors p-1 rounded hover:bg-bg-700">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
            </svg>
          </button>
        </div>

        {/* ── Tabs ───────────────────────────────────────────────────────── */}
        <div className="flex gap-1 px-5 pt-3 shrink-0">
          {(["servers", "rooms"] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={clsx(
                "px-3 py-1.5 rounded text-xs font-medium capitalize transition-colors",
                tab === t
                  ? "bg-accent-500 text-white"
                  : "text-muted hover:bg-bg-700 hover:text-fg"
              )}
            >
              {t}
            </button>
          ))}
        </div>

        {/* ── Body ───────────────────────────────────────────────────────── */}
        <div className="flex-1 overflow-y-auto min-h-0">

          {/* ── Servers tab ──────────────────────────────────────────────── */}
          {tab === "servers" && (
            <div className="p-5">
              {serversLoading ? (
                <LoadingSpinner label="Loading servers…" />
              ) : serversError ? (
                <ErrorBanner message={serversError} />
              ) : servers.length === 0 ? (
                <EmptyState message="No servers known yet. Add a friend from another server to discover new ones." />
              ) : (
                <div className="grid gap-3">
                  {servers.map((srv) => (
                    <ServerCard
                      key={srv.serverName}
                      server={srv}
                      onBrowse={() => browseServer(srv.serverName)}
                    />
                  ))}
                </div>
              )}
            </div>
          )}

          {/* ── Rooms tab ─────────────────────────────────────────────────── */}
          {tab === "rooms" && (
            <div className="flex flex-col gap-3 p-5 h-full">
              {/* Search + filter bar */}
              <div className="flex gap-2 shrink-0">
                <div className="flex-1 relative">
                  <svg
                    width="14" height="14" viewBox="0 0 24 24" fill="currentColor"
                    className="absolute left-3 top-1/2 -translate-y-1/2 text-muted pointer-events-none"
                  >
                    <path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>
                  </svg>
                  <input
                    type="text"
                    value={pendingSearch}
                    onChange={(e) => setPendingSearch(e.target.value)}
                    onKeyDown={(e) => { if (e.key === "Enter") setSearchQuery(pendingSearch); }}
                    placeholder="Search rooms… (Enter to search)"
                    className="w-full pl-8 pr-3 py-1.5 bg-bg-900 border border-bg-600 rounded-lg text-sm text-fg placeholder:text-muted focus:outline-none focus:border-accent-500 transition-colors"
                  />
                </div>
                {/* Server filter chip */}
                {serverFilter && (
                  <div className="flex items-center gap-1 bg-accent-500/20 text-accent-400 border border-accent-500/40 rounded-lg px-3 py-1.5 text-xs font-medium shrink-0">
                    <span className="max-w-[120px] truncate" title={serverFilter}>{serverFilter}</span>
                    <button
                      onClick={() => setServerFilter(null)}
                      className="ml-1 hover:text-fg transition-colors"
                      title="Remove filter"
                    >
                      <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                      </svg>
                    </button>
                  </div>
                )}
              </div>

              {/* Join error */}
              {joinError && (
                <div className="bg-red-500/10 border border-red-500/30 rounded-lg px-3 py-2 text-xs text-red-400 shrink-0">
                  {joinError}
                </div>
              )}

              {/* Room list */}
              {roomsLoading ? (
                <LoadingSpinner label="Loading rooms…" />
              ) : roomsError ? (
                <ErrorBanner message={roomsError} />
              ) : rooms.length === 0 ? (
                <EmptyState
                  message={
                    searchQuery
                      ? `No rooms matching "${searchQuery}"`
                      : serverFilter
                      ? `No public rooms on ${serverFilter}`
                      : "No public federated rooms found yet."
                  }
                />
              ) : (
                <div className="grid gap-2 overflow-y-auto">
                  {rooms.map((room) => (
                    <RoomCard
                      key={room.roomId}
                      room={room}
                      joined={joinedRooms.has(room.roomId)}
                      joining={joiningRoomId === room.roomId}
                      onJoin={() => handleJoin(room.roomId)}
                    />
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* ── Footer ─────────────────────────────────────────────────────── */}
        <div className="px-5 py-3 border-t border-bg-600/50 shrink-0 text-xs text-muted/60">
          Servers listed here are known peers of your home server. Only public rooms can be joined.
        </div>
      </div>
    </div>
  );
}

// ── Sub-components ─────────────────────────────────────────────────────────────

function ServerCard({
  server,
  onBrowse,
}: {
  server: DirectoryServer;
  onBrowse: () => void;
}) {
  const initials = server.serverName.slice(0, 2).toUpperCase();
  return (
    <div className="flex items-center gap-4 bg-bg-700/60 rounded-xl px-4 py-3 border border-bg-600/30 hover:border-bg-500/50 transition-colors">
      {/* Icon */}
      <div className="w-10 h-10 rounded-xl bg-bg-600 flex items-center justify-center shrink-0 overflow-hidden">
        {server.iconUrl ? (
          <img src={server.iconUrl} alt={server.serverName} className="w-full h-full object-cover" />
        ) : (
          <span className="text-xs font-bold text-fg/60">{initials}</span>
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-semibold text-fg truncate">{server.serverName}</div>
        {server.description && (
          <div className="text-xs text-muted truncate mt-0.5">{server.description}</div>
        )}
        <div className="flex items-center gap-3 mt-1">
          <Stat icon="person" value={server.totalUsers} label="users" />
          <Stat icon="chat" value={server.publicRoomCount} label="public rooms" />
        </div>
      </div>

      {/* Buttons */}
      <div className="flex gap-2 shrink-0">
        <button
          onClick={onBrowse}
          className="px-3 py-1.5 rounded-lg text-xs font-medium bg-accent-500/20 text-accent-400 border border-accent-500/40 hover:bg-accent-500/30 transition-colors"
        >
          Browse Rooms
        </button>
      </div>
    </div>
  );
}

function RoomCard({
  room,
  joined,
  joining,
  onJoin,
}: {
  room: DirectoryRoom;
  joined: boolean;
  joining: boolean;
  onJoin: () => void;
}) {
  const displayName = room.name ?? room.roomId;
  return (
    <div className="flex items-center gap-4 bg-bg-700/60 rounded-xl px-4 py-3 border border-bg-600/30 hover:border-bg-500/50 transition-colors">
      {/* Icon */}
      <div className="w-9 h-9 rounded-lg bg-bg-600 flex items-center justify-center shrink-0">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" className="text-fg/50">
          <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z"/>
        </svg>
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-baseline gap-2">
          <span className="text-sm font-semibold text-fg truncate">{displayName}</span>
          <span className="text-[10px] text-muted/60 truncate shrink-0">{room.serverName}</span>
        </div>
        {room.topic && (
          <div className="text-xs text-muted truncate mt-0.5">{room.topic}</div>
        )}
        <div className="flex items-center gap-1 mt-1">
          <Stat icon="person" value={room.memberCount} label="members" />
        </div>
      </div>

      {/* Join button */}
      <div className="shrink-0">
        {joined ? (
          <span className="px-3 py-1.5 rounded-lg text-xs font-medium bg-green-500/20 text-green-400 border border-green-500/30">
            Joined ✓
          </span>
        ) : (
          <button
            onClick={onJoin}
            disabled={joining}
            className={clsx(
              "px-3 py-1.5 rounded-lg text-xs font-medium transition-colors",
              joining
                ? "bg-bg-600 text-muted cursor-not-allowed"
                : "bg-accent-500 text-white hover:bg-accent-500/80"
            )}
          >
            {joining ? "Joining…" : "Join"}
          </button>
        )}
      </div>
    </div>
  );
}

function Stat({
  icon,
  value,
  label,
}: {
  icon: "person" | "chat";
  value: number;
  label: string;
}) {
  return (
    <span className="flex items-center gap-0.5 text-[10px] text-muted/70">
      {icon === "person" ? (
        <svg width="9" height="9" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
        </svg>
      ) : (
        <svg width="9" height="9" viewBox="0 0 24 24" fill="currentColor">
          <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z"/>
        </svg>
      )}
      <span>{value.toLocaleString()} {label}</span>
    </span>
  );
}

function LoadingSpinner({ label }: { label: string }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 py-16 text-muted">
      <svg
        className="animate-spin"
        width="24" height="24" viewBox="0 0 24 24" fill="none"
        stroke="currentColor" strokeWidth="2"
      >
        <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"
          strokeLinecap="round"/>
      </svg>
      <span className="text-xs">{label}</span>
    </div>
  );
}

function ErrorBanner({ message }: { message: string }) {
  return (
    <div className="bg-red-500/10 border border-red-500/30 rounded-lg px-4 py-3 text-xs text-red-400">
      {message}
    </div>
  );
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 py-16 text-center text-muted">
      <svg width="40" height="40" viewBox="0 0 24 24" fill="currentColor" className="opacity-30">
        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
      </svg>
      <p className="text-sm max-w-xs">{message}</p>
    </div>
  );
}

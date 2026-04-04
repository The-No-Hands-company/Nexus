import { create } from "zustand";
import { persist } from "zustand/middleware";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface Session {
  accessToken: string;
  refreshToken: string;
  username: string;
  userId: string;
  avatar: string | null;
  serverUrl: string;
}

export interface NxServer {
  id: string;
  name: string;
  icon: string | null;
  description: string | null;
  owner_id: string;
}

export interface NxChannel {
  id: string;
  name: string | null;
  kind: string;
  server_id: string | null;
  topic: string | null;
  is_e2ee: boolean;
  recipients?: { id: string; username: string; avatar: string | null }[];
}

export interface NxMessage {
  id: string;
  content: string;
  author_id: string;
  author_username: string | null;
  author_avatar: string | null;
  channel_id: string;
  created_at: string;
  edited_at: string | null;
  attachments: Attachment[];
  reactions: Reaction[];
  reply_to: string | null;
}

export interface Attachment {
  id: string;
  filename: string;
  content_type: string;
  size: number;
  url: string | null;
}

export interface Reaction {
  emoji: string;
  count: number;
  me: boolean;
}

export interface NxUser {
  id: string;
  username: string;
  display_name: string | null;
  avatar: string | null;
  presence: "online" | "idle" | "dnd" | "invisible" | "offline";
  status: string | null;
}

export type GatewayStatus = "offline" | "connecting" | "online";

// ── API helpers ───────────────────────────────────────────────────────────────

export function apiBase(session: Session | null): string {
  return (session?.serverUrl ?? localStorage.getItem("nexus_server_url") ?? "http://localhost:8080") + "/api/v1";
}

export function authHeaders(session: Session | null): HeadersInit {
  const h: Record<string, string> = { "Content-Type": "application/json" };
  if (session) h["Authorization"] = `Bearer ${session.accessToken}`;
  return h;
}

async function apiFetch<T>(
  session: Session,
  path: string,
  init: RequestInit = {}
): Promise<T> {
  const res = await fetch(`${apiBase(session)}${path}`, {
    ...init,
    headers: { ...authHeaders(session), ...(init.headers ?? {}) },
  });
  if (!res.ok) {
    const text = await res.text().catch(() => String(res.status));
    throw new Error(text || `HTTP ${res.status}`);
  }
  return res.json() as Promise<T>;
}

// ── Store ─────────────────────────────────────────────────────────────────────

interface Store {
  // Auth
  session: Session | null;
  setSession: (s: Session | null) => void;

  // Server
  serverUrl: string;
  setServerUrl: (url: string) => void;

  // Navigation
  activeServerId: string | null;
  activeChannelId: string | null;
  setActiveServer: (id: string | null) => void;
  setActiveChannel: (id: string | null) => void;

  // Data
  servers: NxServer[];
  channels: NxChannel[];
  dmChannels: NxChannel[];
  messages: Record<string, NxMessage[]>;
  unread: Record<string, boolean>;
  typing: Record<string, string[]>; // channelId -> usernames
  onlineUsers: Record<string, NxUser["presence"]>;
  gatewayStatus: GatewayStatus;
  setGatewayStatus: (s: GatewayStatus) => void;

  // Loaders
  loadServers: () => Promise<void>;
  loadChannels: (serverId: string) => Promise<void>;
  loadDms: () => Promise<void>;
  loadMessages: (channelId: string) => Promise<void>;
  loadMoreMessages: (channelId: string) => Promise<void>;

  // Actions
  sendMessage: (channelId: string, content: string, replyTo?: string) => Promise<void>;
  addReaction: (channelId: string, messageId: string, emoji: string) => Promise<void>;
  deleteMessage: (channelId: string, messageId: string) => Promise<void>;
  createServer: (name: string) => Promise<NxServer>;
  createChannel: (serverId: string, name: string, kind: string) => Promise<NxChannel>;

  // Gateway events (called by the gateway hook)
  onMessageCreate: (msg: NxMessage) => void;
  onMessageDelete: (channelId: string, messageId: string) => void;
  onPresenceUpdate: (userId: string, presence: NxUser["presence"]) => void;
  onTypingStart: (channelId: string, username: string) => void;
  markRead: (channelId: string) => void;
}

export const useStore = create<Store>()(
  persist(
    (set, get) => ({
      session: null,
      setSession: (session) => set({ session }),
      serverUrl: "http://localhost:8080",
      setServerUrl: (serverUrl) => {
        localStorage.setItem("nexus_server_url", serverUrl);
        set({ serverUrl });
      },

      activeServerId: null,
      activeChannelId: null,
      setActiveServer: (id) => set({ activeServerId: id, activeChannelId: null }),
      setActiveChannel: (id) => {
        set({ activeChannelId: id });
        if (id) get().markRead(id);
      },

      servers: [],
      channels: [],
      dmChannels: [],
      messages: {},
      unread: {},
      typing: {},
      onlineUsers: {},
      gatewayStatus: "offline",
      setGatewayStatus: (gatewayStatus) => set({ gatewayStatus }),

      // ── Loaders ──────────────────────────────────────────────────────────

      loadServers: async () => {
        const s = get().session;
        if (!s) return;
        try {
          const data = await apiFetch<NxServer[]>(s, "/servers");
          set({ servers: Array.isArray(data) ? data : [] });
        } catch (e) { console.error("loadServers", e); }
      },

      loadChannels: async (serverId) => {
        const s = get().session;
        if (!s) return;
        try {
          const data = await apiFetch<NxChannel[]>(s, `/servers/${serverId}/channels`);
          set({ channels: Array.isArray(data) ? data : [] });
        } catch (e) { console.error("loadChannels", e); }
      },

      loadDms: async () => {
        const s = get().session;
        if (!s) return;
        try {
          const data = await apiFetch<NxChannel[]>(s, "/users/@me/channels");
          set({ dmChannels: Array.isArray(data) ? data : [] });
        } catch (e) { console.error("loadDms", e); }
      },

      loadMessages: async (channelId) => {
        const s = get().session;
        if (!s) return;
        try {
          const data = await apiFetch<NxMessage[]>(
            s, `/channels/${channelId}/messages?limit=50`
          );
          set((st) => ({
            messages: { ...st.messages, [channelId]: Array.isArray(data) ? data : [] },
          }));
        } catch (e) { console.error("loadMessages", e); }
      },

      loadMoreMessages: async (channelId) => {
        const s = get().session;
        if (!s) return;
        const existing = get().messages[channelId] ?? [];
        const oldest = existing[existing.length - 1];
        if (!oldest) return;
        try {
          const data = await apiFetch<NxMessage[]>(
            s, `/channels/${channelId}/messages?limit=50&before=${oldest.id}`
          );
          if (!Array.isArray(data) || data.length === 0) return;
          set((st) => ({
            messages: {
              ...st.messages,
              [channelId]: [...(st.messages[channelId] ?? []), ...data],
            },
          }));
        } catch (e) { console.error("loadMoreMessages", e); }
      },

      // ── Actions ───────────────────────────────────────────────────────────

      sendMessage: async (channelId, content, replyTo) => {
        const s = get().session;
        if (!s) return;
        await apiFetch(s, `/channels/${channelId}/messages`, {
          method: "POST",
          body: JSON.stringify({ content, reply_to_message_id: replyTo ?? null }),
        });
      },

      addReaction: async (channelId, messageId, emoji) => {
        const s = get().session;
        if (!s) return;
        await apiFetch(s, `/channels/${channelId}/messages/${messageId}/reactions/${encodeURIComponent(emoji)}`, {
          method: "PUT",
        });
      },

      deleteMessage: async (channelId, messageId) => {
        const s = get().session;
        if (!s) return;
        await apiFetch(s, `/channels/${channelId}/messages/${messageId}`, { method: "DELETE" });
        set((st) => ({
          messages: {
            ...st.messages,
            [channelId]: (st.messages[channelId] ?? []).filter((m) => m.id !== messageId),
          },
        }));
      },

      createServer: async (name) => {
        const s = get().session;
        if (!s) throw new Error("Not authenticated");
        const srv = await apiFetch<NxServer>(s, "/servers", {
          method: "POST",
          body: JSON.stringify({ name }),
        });
        set((st) => ({ servers: [srv, ...st.servers] }));
        return srv;
      },

      createChannel: async (serverId, name, kind = "text") => {
        const s = get().session;
        if (!s) throw new Error("Not authenticated");
        const ch = await apiFetch<NxChannel>(s, `/servers/${serverId}/channels`, {
          method: "POST",
          body: JSON.stringify({ name, kind }),
        });
        set((st) => ({ channels: [ch, ...st.channels] }));
        return ch;
      },

      // ── Gateway event handlers ─────────────────────────────────────────────

      onMessageCreate: (msg) => {
        set((st) => {
          const prev = st.messages[msg.channel_id] ?? [];
          if (prev.some((m) => m.id === msg.id)) return {};
          const updated = [msg, ...prev];
          const isActive = st.activeChannelId === msg.channel_id;
          return {
            messages: { ...st.messages, [msg.channel_id]: updated },
            unread: isActive ? st.unread : { ...st.unread, [msg.channel_id]: true },
          };
        });
      },

      onMessageDelete: (channelId, messageId) => {
        set((st) => ({
          messages: {
            ...st.messages,
            [channelId]: (st.messages[channelId] ?? []).filter((m) => m.id !== messageId),
          },
        }));
      },

      onPresenceUpdate: (userId, presence) => {
        set((st) => ({ onlineUsers: { ...st.onlineUsers, [userId]: presence } }));
      },

      onTypingStart: (channelId, username) => {
        set((st) => {
          const cur = st.typing[channelId] ?? [];
          if (cur.includes(username)) return {};
          const next = [...cur, username];
          setTimeout(() => {
            set((s2) => ({
              typing: {
                ...s2.typing,
                [channelId]: (s2.typing[channelId] ?? []).filter((u) => u !== username),
              },
            }));
          }, 5000);
          return { typing: { ...st.typing, [channelId]: next } };
        });
      },

      markRead: (channelId) => {
        set((st) => ({ unread: { ...st.unread, [channelId]: false } }));
      },
    }),
    {
      name: "nexus-web-session",
      partialize: (s) => ({ session: s.session, serverUrl: s.serverUrl }),
    }
  )
);

// ── Nexus Mobile — Zustand Store ──────────────────────────────────────────────
//
// Mirrors the nexus-web store API so components can stay structurally similar,
// but uses AsyncStorage for persistence instead of localStorage and React Native
// patterns throughout.

import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import AsyncStorage from "@react-native-async-storage/async-storage";
import { apiFetch, apiPost, apiDelete, apiBase } from "./api";
import type {
  Session,
  NxServer,
  NxChannel,
  NxMessage,
  NxUser,
  GatewayStatus,
} from "./types";

// ── Store interface ────────────────────────────────────────────────────────────

interface Store {
  // ── Auth ──────────────────────────────────────────────────────────────────
  session: Session | null;
  setSession: (s: Session | null) => void;

  // ── Navigation state ──────────────────────────────────────────────────────
  activeServerId: string | null;
  activeChannelId: string | null;
  setActiveServer: (id: string | null) => void;
  setActiveChannel: (id: string | null) => void;

  // ── Data ──────────────────────────────────────────────────────────────────
  servers: NxServer[];
  channels: NxChannel[];
  dmChannels: NxChannel[];
  messages: Record<string, NxMessage[]>;
  users: Record<string, NxUser>;
  unread: Record<string, boolean>;
  typing: Record<string, string[]>; // channelId → [username, ...]

  // ── Gateway ───────────────────────────────────────────────────────────────
  gatewayStatus: GatewayStatus;
  setGatewayStatus: (s: GatewayStatus) => void;

  // ── Pagination ────────────────────────────────────────────────────────────
  hasMoreMessages: Record<string, boolean>;

  // ── Loaders ───────────────────────────────────────────────────────────────
  loadServers: () => Promise<void>;
  loadChannels: (serverId: string) => Promise<void>;
  loadDms: () => Promise<void>;
  loadMessages: (channelId: string) => Promise<void>;
  loadMoreMessages: (channelId: string) => Promise<void>;

  // ── Actions ───────────────────────────────────────────────────────────────
  sendMessage: (channelId: string, content: string, replyTo?: string | null) => Promise<void>;
  deleteMessage: (channelId: string, messageId: string) => Promise<void>;
  addReaction: (channelId: string, messageId: string, emoji: string) => Promise<void>;
  createServer: (name: string) => Promise<NxServer>;
  createChannel: (serverId: string, name: string, kind?: string) => Promise<NxChannel>;
  createDm: (userId: string) => Promise<NxChannel>;
  markRead: (channelId: string) => void;
  sendTyping: (channelId: string) => Promise<void>;

  // ── Gateway event handlers (called by useGateway hook) ────────────────────
  onMessageCreate: (msg: NxMessage) => void;
  onMessageDelete: (channelId: string, messageId: string) => void;
  onPresenceUpdate: (userId: string, status: NxUser["presence"]) => void;
  onTypingStart: (channelId: string, username: string) => void;
}

// ── Implementation ────────────────────────────────────────────────────────────

const PAGE_SIZE = 50;

export const useStore = create<Store>()(
  persist(
    (set, get) => ({
      // ── Auth ────────────────────────────────────────────────────────────────
      session: null,
      setSession: (session) =>
        set({
          session,
          servers: [],
          channels: [],
          dmChannels: [],
          messages: {},
          unread: {},
          activeServerId: null,
          activeChannelId: null,
          gatewayStatus: "offline",
        }),

      // ── Navigation ──────────────────────────────────────────────────────────
      activeServerId: null,
      activeChannelId: null,
      setActiveServer: (id) => set({ activeServerId: id, activeChannelId: null }),
      setActiveChannel: (id) => set({ activeChannelId: id }),

      // ── Data ────────────────────────────────────────────────────────────────
      servers: [],
      channels: [],
      dmChannels: [],
      messages: {},
      users: {},
      unread: {},
      typing: {},
      hasMoreMessages: {},
      gatewayStatus: "offline",
      setGatewayStatus: (gatewayStatus) => set({ gatewayStatus }),

      // ── Loaders ─────────────────────────────────────────────────────────────

      loadServers: async () => {
        const { session } = get();
        if (!session) return;
        try {
          const data = await apiFetch<NxServer[]>(session, "/servers");
          set({ servers: data });
        } catch (e) {
          console.warn("[store] loadServers failed:", e);
        }
      },

      loadChannels: async (serverId) => {
        const { session } = get();
        if (!session) return;
        try {
          const data = await apiFetch<NxChannel[]>(session, `/servers/${serverId}/channels`);
          const sorted = [...data].sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
          set({ channels: sorted });
        } catch (e) {
          console.warn("[store] loadChannels failed:", e);
        }
      },

      loadDms: async () => {
        const { session } = get();
        if (!session) return;
        try {
          const data = await apiFetch<NxChannel[]>(session, "/channels/@me/dms");
          set({ dmChannels: data });
        } catch (e) {
          console.warn("[store] loadDms failed:", e);
        }
      },

      loadMessages: async (channelId) => {
        const { session } = get();
        if (!session) return;
        try {
          const data = await apiFetch<NxMessage[]>(
            session,
            `/channels/${channelId}/messages?limit=${PAGE_SIZE}`
          );
          set((s) => ({
            messages: { ...s.messages, [channelId]: data.reverse() },
            hasMoreMessages: { ...s.hasMoreMessages, [channelId]: data.length === PAGE_SIZE },
          }));
        } catch (e) {
          console.warn("[store] loadMessages failed:", e);
        }
      },

      loadMoreMessages: async (channelId) => {
        const { session, messages, hasMoreMessages } = get();
        if (!session || !hasMoreMessages[channelId]) return;
        const oldest = messages[channelId]?.[0];
        if (!oldest) return;
        try {
          const data = await apiFetch<NxMessage[]>(
            session,
            `/channels/${channelId}/messages?limit=${PAGE_SIZE}&before=${oldest.id}`
          );
          set((s) => ({
            messages: {
              ...s.messages,
              [channelId]: [...data.reverse(), ...(s.messages[channelId] ?? [])],
            },
            hasMoreMessages: { ...s.hasMoreMessages, [channelId]: data.length === PAGE_SIZE },
          }));
        } catch (e) {
          console.warn("[store] loadMoreMessages failed:", e);
        }
      },

      // ── Actions ──────────────────────────────────────────────────────────────

      sendMessage: async (channelId, content, replyTo) => {
        const { session } = get();
        if (!session) return;
        const msg = await apiPost<NxMessage>(session, `/channels/${channelId}/messages`, {
          content,
          reference: replyTo ? { message_id: replyTo, channel_id: channelId } : undefined,
        });
        set((s) => ({
          messages: {
            ...s.messages,
            [channelId]: [...(s.messages[channelId] ?? []), msg],
          },
        }));
      },

      deleteMessage: async (channelId, messageId) => {
        const { session } = get();
        if (!session) return;
        await apiDelete(session, `/channels/${channelId}/messages/${messageId}`);
        set((s) => ({
          messages: {
            ...s.messages,
            [channelId]: (s.messages[channelId] ?? []).filter((m) => m.id !== messageId),
          },
        }));
      },

      addReaction: async (channelId, messageId, emoji) => {
        const { session } = get();
        if (!session) return;
        await apiFetch(session, `/channels/${channelId}/messages/${messageId}/reactions/${encodeURIComponent(emoji)}`, {
          method: "PUT",
        });
      },

      createServer: async (name) => {
        const { session } = get();
        if (!session) throw new Error("Not authenticated");
        const srv = await apiPost<NxServer>(session, "/servers", { name });
        set((s) => ({ servers: [...s.servers, srv] }));
        return srv;
      },

      createChannel: async (serverId, name, kind = "text") => {
        const { session } = get();
        if (!session) throw new Error("Not authenticated");
        const ch = await apiPost<NxChannel>(session, `/servers/${serverId}/channels`, { name, kind });
        set((s) => ({ channels: [...s.channels, ch] }));
        return ch;
      },

      createDm: async (userId) => {
        const { session } = get();
        if (!session) throw new Error("Not authenticated");
        const dm = await apiPost<NxChannel>(session, "/channels/@me/dms", { recipient_id: userId });
        set((s) => ({
          dmChannels: s.dmChannels.some((d) => d.id === dm.id)
            ? s.dmChannels
            : [dm, ...s.dmChannels],
        }));
        return dm;
      },

      markRead: (channelId) =>
        set((s) => ({ unread: { ...s.unread, [channelId]: false } })),

      sendTyping: async (channelId) => {
        const { session } = get();
        if (!session) return;
        await apiFetch(session, `/channels/${channelId}/typing`, { method: "POST" }).catch(() => {});
      },

      // ── Gateway event handlers ────────────────────────────────────────────────

      onMessageCreate: (msg) => {
        const { activeChannelId } = get();
        set((s) => {
          const existing = s.messages[msg.channel_id] ?? [];
          // Deduplicate by ID (gateway may echo our own sends)
          if (existing.some((m) => m.id === msg.id)) return {};
          return {
            messages: { ...s.messages, [msg.channel_id]: [...existing, msg] },
            unread: msg.channel_id !== activeChannelId
              ? { ...s.unread, [msg.channel_id]: true }
              : s.unread,
          };
        });
      },

      onMessageDelete: (channelId, messageId) =>
        set((s) => ({
          messages: {
            ...s.messages,
            [channelId]: (s.messages[channelId] ?? []).filter((m) => m.id !== messageId),
          },
        })),

      onPresenceUpdate: (userId, status) =>
        set((s) => ({
          users: {
            ...s.users,
            [userId]: { ...(s.users[userId] ?? { id: userId, username: "", display_name: null, avatar: null, status: null }), presence: status },
          },
        })),

      onTypingStart: (channelId, username) => {
        set((s) => {
          const current = s.typing[channelId] ?? [];
          if (current.includes(username)) return {};
          const next = [...current, username];
          // Auto-remove after 5 seconds
          setTimeout(() => {
            set((st) => ({
              typing: {
                ...st.typing,
                [channelId]: (st.typing[channelId] ?? []).filter((u) => u !== username),
              },
            }));
          }, 5_000);
          return { typing: { ...s.typing, [channelId]: next } };
        });
      },
    }),
    {
      name: "nexus-mobile-store",
      storage: createJSONStorage(() => AsyncStorage),
      // Only persist auth session and server URL — the rest is fetched fresh on mount
      partialize: (s) => ({
        session: s.session,
      }),
    }
  )
);

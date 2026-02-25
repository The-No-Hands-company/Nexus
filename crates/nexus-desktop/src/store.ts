import { create } from "zustand";
import { invoke } from "./invoke";
import type { PluginManifest } from "./plugins/types";
import { DEFAULT_THEME_ID } from "./themes/themes";

export interface Session {
  userId: string;
  username: string;
  displayName?: string;
  avatar?: string;
  serverUrl: string;
  accessToken: string;
}

export interface Server {
  id: string;
  name: string;
  icon?: string;
  ownerId: string;
}

export interface Channel {
  id: string;
  serverId: string;
  name: string;
  kind: "text" | "voice" | "announcement";
  isE2ee?: boolean;
}

export interface Message {
  id: string;
  channelId: string;
  authorId: string;
  authorUsername: string;
  content: string;
  createdAt: string;
  editedAt?: string;
  attachments?: Attachment[];
  reactions?: Reaction[];
  embeds?: Embed[];
  replyTo?: string;
  threadId?: string;
}

export interface Attachment {
  id: string;
  filename: string;
  url: string;
  size: number;
  contentType?: string;
}

export interface Reaction {
  emoji: string;
  count: number;
  /** true if the current user has added this reaction */
  me: boolean;
}

export interface Embed {
  type?: string;
  title?: string;
  description?: string;
  url?: string;
  thumbnail?: { url: string };
  image?: { url: string };
  color?: number;
}

export interface VoiceParticipant {
  userId: string;
  username: string;
  speaking: boolean;
  muted: boolean;
  deafened: boolean;
  avatar?: string;
}

export interface Relationship {
  /** The other user's ID */
  id: string;
  direction: "incoming" | "outgoing";
  status: "pending" | "accepted" | "blocked" | "denied";
  user: {
    id: string;
    username: string;
    displayName?: string;
    avatar?: string;
  };
}

export interface DmChannel {
  id: string;
  channelType: "dm" | "group_dm";
  name?: string;
  lastMessageId?: string;
  recipients: {
    id: string;
    username: string;
    displayName?: string;
    avatar?: string;
  }[];
}

export interface ServerMember {
  userId: string;
  username: string;
  displayName?: string;
  avatar?: string;
  nickname?: string;
  presence: "online" | "idle" | "do_not_disturb" | "invisible" | "offline";
  joinedAt: string;
  roles: string[];
  muted: boolean;
  deafened: boolean;
}

export interface UpdateInfo {
  version: string;
  body: string;
}

interface StoreState {
  // Auth
  session: Session | null;
  setSession: (session: Session | null) => void;

  // Servers
  servers: Server[];
  activeServerId: string | null;
  setServers: (servers: Server[]) => void;
  setActiveServer: (id: string | null) => void;

  // Channels
  channels: Channel[];
  activeChannelId: string | null;
  setChannels: (channels: Channel[]) => void;
  setActiveChannel: (id: string | null) => void;

  // Messages — keyed by channelId
  messages: Record<string, Message[]>;
  appendMessage: (channelId: string, msg: Message) => void;
  prependMessages: (channelId: string, msgs: Message[]) => void;
  setMessages: (channelId: string, msgs: Message[]) => void;
  updateMessageReaction: (channelId: string, messageId: string, emoji: string, delta: number, mine: boolean) => void;

  // Typing — keyed by channelId → list of usernames currently typing
  typingUsers: Record<string, string[]>;
  setTyping: (channelId: string, username: string, active: boolean) => void;

  // Unread — channelIds that have received messages since last viewed
  unreadChannels: Record<string, boolean>;

  // Voice
  voiceParticipants: VoiceParticipant[];
  joinedVoiceChannelId: string | null;
  pttActive: boolean;
  setVoiceParticipants: (participants: VoiceParticipant[]) => void;
  setJoinedVoiceChannel: (id: string | null) => void;
  setPttActive: (active: boolean) => void;

  // UI
  updateAvailable: UpdateInfo | null;
  setUpdateAvailable: (info: UpdateInfo | null) => void;
  sidebarCollapsed: boolean;
  setSidebarCollapsed: (v: boolean) => void;

  // Theme
  activeThemeId: string;
  setActiveTheme: (id: string) => void;

  // Plugins
  plugins: PluginManifest[];
  enabledPlugins: string[];
  installPlugin: (manifest: PluginManifest) => void;
  uninstallPlugin: (id: string) => void;
  togglePlugin: (id: string) => void;

  // Actions
  logout: () => Promise<void>;
  loadServers: () => Promise<void>;
  loadChannels: (serverId: string) => Promise<void>;
  loadMessages: (channelId: string, before?: string) => Promise<void>;
  createChannel: (serverId: string, name: string, channelType: string) => Promise<Channel>;

  // Home / Friends / DMs
  isHomeMode: boolean;
  setHomeMode: (v: boolean) => void;
  relationships: Relationship[];
  setRelationships: (rels: Relationship[]) => void;
  dmChannels: DmChannel[];
  setDmChannels: (dms: DmChannel[]) => void;
  appendDmChannel: (dm: DmChannel) => void;
  loadRelationships: () => Promise<void>;
  loadDmChannels: () => Promise<void>;

  // Server members — keyed by serverId
  members: Record<string, ServerMember[]>;
  loadMembers: (serverId: string) => Promise<void>;
}

// Module-level map so typing-clear timeouts survive re-renders
const _typingTimers = new Map<string, ReturnType<typeof setTimeout>>();

export const useStore = create<StoreState>((set, get) => ({
  // ─── Auth ─────────────────────────────────────────────────────────────
  session: null,
  setSession: (session) => set({ session }),

  // ─── Servers ──────────────────────────────────────────────────────────
  servers: [],
  activeServerId: null,
  setServers: (servers) => set({ servers }),
  setActiveServer: (id) => set({ activeServerId: id, channels: [], activeChannelId: null }),

  // ─── Channels ─────────────────────────────────────────────────────────
  channels: [],
  activeChannelId: null,
  setChannels: (channels) => set({ channels }),
  setActiveChannel: (id) => set((s) => {
    const unreadChannels = { ...s.unreadChannels };
    if (id) delete unreadChannels[id];
    return { activeChannelId: id, unreadChannels };
  }),

  // ─── Messages ─────────────────────────────────────────────────────────
  messages: {},
  appendMessage: (channelId, msg) =>
    set((s) => {
      const existing = s.messages[channelId] ?? [];
      // Dedup: if message already present (optimistic add + WS event), skip
      if (existing.some((m) => m.id === msg.id)) return s;
      return {
        messages: {
          ...s.messages,
          [channelId]: [...existing, msg],
        },
        // Mark channel unread if the user isn't currently looking at it
        unreadChannels: s.activeChannelId === channelId
          ? s.unreadChannels
          : { ...s.unreadChannels, [channelId]: true },
      };
    }),
  prependMessages: (channelId, msgs) =>
    set((s) => ({
      messages: {
        ...s.messages,
        [channelId]: [...msgs, ...(s.messages[channelId] ?? [])],
      },
    })),
  setMessages: (channelId, msgs) =>
    set((s) => ({
      messages: { ...s.messages, [channelId]: msgs },
    })),

  updateMessageReaction: (channelId, messageId, emoji, delta, mine) =>
    set((s) => {
      const msgs = s.messages[channelId];
      if (!msgs) return s;
      return {
        messages: {
          ...s.messages,
          [channelId]: msgs.map((m) => {
            if (m.id !== messageId) return m;
            const existing = m.reactions ?? [];
            const idx = existing.findIndex((r) => r.emoji === emoji);
            if (idx >= 0) {
              const updated = [...existing];
              const newCount = updated[idx].count + delta;
              if (newCount <= 0) {
                updated.splice(idx, 1);
              } else {
                updated[idx] = {
                  ...updated[idx],
                  count: newCount,
                  me: delta > 0 ? mine || updated[idx].me : (mine ? false : updated[idx].me),
                };
              }
              return { ...m, reactions: updated };
            } else if (delta > 0) {
              return { ...m, reactions: [...existing, { emoji, count: 1, me: mine }] };
            }
            return m;
          }),
        },
      };
    }),

  // ─── Typing ───────────────────────────────────────────────────────────────
  typingUsers: {},
  setTyping: (channelId, username, active) => {
    const key = `${channelId}:${username}`;
    if (active) {
      set((s) => ({
        typingUsers: {
          ...s.typingUsers,
          [channelId]: [...new Set([...(s.typingUsers[channelId] ?? []), username])],
        },
      }));
      if (_typingTimers.has(key)) clearTimeout(_typingTimers.get(key)!);
      _typingTimers.set(key, setTimeout(() => {
        set((s) => ({
          typingUsers: {
            ...s.typingUsers,
            [channelId]: (s.typingUsers[channelId] ?? []).filter((u) => u !== username),
          },
        }));
        _typingTimers.delete(key);
      }, 6000));
    } else {
      if (_typingTimers.has(key)) { clearTimeout(_typingTimers.get(key)!); _typingTimers.delete(key); }
      set((s) => ({
        typingUsers: {
          ...s.typingUsers,
          [channelId]: (s.typingUsers[channelId] ?? []).filter((u) => u !== username),
        },
      }));
    }
  },

  // ─── Unread ───────────────────────────────────────────────────────────────
  unreadChannels: {},

  // ─── Voice ────────────────────────────────────────────────────────────
  voiceParticipants: [],
  joinedVoiceChannelId: null,
  pttActive: false,
  setVoiceParticipants: (participants) => set({ voiceParticipants: participants }),
  setJoinedVoiceChannel: (id) => set({ joinedVoiceChannelId: id }),
  setPttActive: (active) => set({ pttActive: active }),

  // ─── UI ───────────────────────────────────────────────────────────────
  updateAvailable: null,
  setUpdateAvailable: (info) => set({ updateAvailable: info }),
  sidebarCollapsed: false,
  setSidebarCollapsed: (v) => set({ sidebarCollapsed: v }),

  // ─── Theme ────────────────────────────────────────────────────────────
  activeThemeId: localStorage.getItem("nexus:theme") ?? DEFAULT_THEME_ID,
  setActiveTheme: (id) => {
    localStorage.setItem("nexus:theme", id);
    set({ activeThemeId: id });
  },

  // ─── Plugins ──────────────────────────────────────────────────────────
  plugins: (() => {
    try {
      return JSON.parse(localStorage.getItem("nexus:plugins") ?? "[]") as PluginManifest[];
    } catch { return []; }
  })(),
  enabledPlugins: (() => {
    try {
      return JSON.parse(localStorage.getItem("nexus:enabled-plugins") ?? "[]") as string[];
    } catch { return []; }
  })(),
  installPlugin: (manifest) =>
    set((s) => {
      const plugins = [...s.plugins.filter((p) => p.id !== manifest.id), manifest];
      localStorage.setItem("nexus:plugins", JSON.stringify(plugins));
      return { plugins };
    }),
  uninstallPlugin: (id) =>
    set((s) => {
      const plugins = s.plugins.filter((p) => p.id !== id);
      const enabledPlugins = s.enabledPlugins.filter((i) => i !== id);
      localStorage.setItem("nexus:plugins", JSON.stringify(plugins));
      localStorage.setItem("nexus:enabled-plugins", JSON.stringify(enabledPlugins));
      return { plugins, enabledPlugins };
    }),
  togglePlugin: (id) =>
    set((s) => {
      const enabledPlugins = s.enabledPlugins.includes(id)
        ? s.enabledPlugins.filter((i) => i !== id)
        : [...s.enabledPlugins, id];
      localStorage.setItem("nexus:enabled-plugins", JSON.stringify(enabledPlugins));
      return { enabledPlugins };
    }),

  // ─── Actions ──────────────────────────────────────────────────────────
  logout: async () => {
    try {
      await invoke("logout");
    } catch {
      // ignore
    }
    _typingTimers.forEach(clearTimeout);
    _typingTimers.clear();
    set({
      session: null,
      servers: [],
      channels: [],
      messages: {},
      typingUsers: {},
      unreadChannels: {},
      activeServerId: null,
      activeChannelId: null,
      isHomeMode: false,
      relationships: [],
      dmChannels: [],
      members: {},
    });
  },

  loadServers: async () => {
    try {
      const servers = await invoke<Server[]>("list_servers");
      set({ servers });
      // Auto-select the first server if none is currently active
      if (servers.length > 0 && !get().activeServerId) {
        get().setActiveServer(servers[0].id);
      }
    } catch (e) {
      // If the token expired and couldn't be refreshed, force logout
      if (e instanceof Error && e.message.startsWith("401:")) {
        console.warn("Session expired — logging out");
        get().logout();
        return;
      }
      console.error("loadServers error", e);
    }
  },

  loadChannels: async (serverId: string) => {
    try {
      const channels = await invoke<Channel[]>("list_channels", { serverId });
      set({ channels });
    } catch (e) {
      console.error("loadChannels error", e);
    }
  },

  createChannel: async (serverId: string, name: string, channelType: string): Promise<Channel> => {
    const ch = await invoke<Channel>("create_channel", { serverId, name, channelType });
    set((s) => ({ channels: [...s.channels, ch] }));
    return ch;
  },

  loadMessages: async (channelId: string, before?: string) => {
    try {
      const msgs = await invoke<Message[]>("fetch_history", {
        channelId,
        before: before ?? null,
        limit: 50,
      });
      if (before) {
        get().prependMessages(channelId, msgs);
      } else {
        get().setMessages(channelId, msgs);
      }
    } catch (e) {
      console.error("loadMessages error", e);
    }
  },

  // ─── Home / Friends / DMs ────────────────────────────────────────────
  isHomeMode: false,
  setHomeMode: (v) => set({ isHomeMode: v }),
  relationships: [],
  setRelationships: (rels) => set({ relationships: rels }),
  dmChannels: [],
  setDmChannels: (dms) => set({ dmChannels: dms }),
  appendDmChannel: (dm) =>
    set((s) => {
      if (s.dmChannels.some((d) => d.id === dm.id)) return s;
      return { dmChannels: [dm, ...s.dmChannels] };
    }),

  loadRelationships: async () => {
    try {
      const rels = await invoke<Relationship[]>("list_relationships");
      set({ relationships: rels });
    } catch (e) {
      console.error("loadRelationships error", e);
    }
  },

  loadDmChannels: async () => {
    try {
      const dms = await invoke<DmChannel[]>("list_dm_channels");
      set({ dmChannels: dms });
    } catch (e) {
      console.error("loadDmChannels error", e);
    }
  },

  // ─── Server members ───────────────────────────────────────────────────
  members: {},
  loadMembers: async (serverId: string) => {
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const raw = await invoke<any[]>("list_members", { serverId });
      const mapped: ServerMember[] = raw.map((m) => ({
        userId: m.user_id,
        username: m.username,
        displayName: m.display_name ?? undefined,
        avatar: m.avatar ?? undefined,
        nickname: m.nickname ?? undefined,
        presence: m.presence ?? "offline",
        joinedAt: m.joined_at,
        roles: [],
        muted: m.muted ?? false,
        deafened: m.deafened ?? false,
      }));
      set((s) => ({ members: { ...s.members, [serverId]: mapped } }));
    } catch (e) {
      console.error("loadMembers error", e);
    }
  },
}));

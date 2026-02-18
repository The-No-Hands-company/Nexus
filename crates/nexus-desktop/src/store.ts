import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface Session {
  userId: string;
  username: string;
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
  replyTo?: string;
}

export interface Attachment {
  id: string;
  filename: string;
  url: string;
  size: number;
  contentType?: string;
}

export interface VoiceParticipant {
  userId: string;
  username: string;
  speaking: boolean;
  muted: boolean;
  deafened: boolean;
  avatar?: string;
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

  // Actions
  logout: () => Promise<void>;
  loadServers: () => Promise<void>;
  loadChannels: (serverId: string) => Promise<void>;
  loadMessages: (channelId: string, before?: string) => Promise<void>;
}

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
  setActiveChannel: (id) => set({ activeChannelId: id }),

  // ─── Messages ─────────────────────────────────────────────────────────
  messages: {},
  appendMessage: (channelId, msg) =>
    set((s) => ({
      messages: {
        ...s.messages,
        [channelId]: [...(s.messages[channelId] ?? []), msg],
      },
    })),
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

  // ─── Actions ──────────────────────────────────────────────────────────
  logout: async () => {
    try {
      await invoke("logout");
    } catch {
      // ignore
    }
    set({
      session: null,
      servers: [],
      channels: [],
      messages: {},
      activeServerId: null,
      activeChannelId: null,
    });
  },

  loadServers: async () => {
    try {
      const servers = await invoke<Server[]>("list_servers");
      set({ servers });
    } catch (e) {
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
}));

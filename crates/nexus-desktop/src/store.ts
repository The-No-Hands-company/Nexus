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
  require2fa?: boolean;
  spamWindowSecs?: number;
  spamMaxMessages?: number;
  /** v0.15 — current supporter tier (0-3) */
  boostTier?: number;
  /** v0.15 — active booster count */
  boosterCount?: number;
  /** v0.15 — vanity invite code */
  vanityCode?: string;
}

export interface Channel {
  id: string;
  serverId: string;
  name: string;
  kind: "text" | "voice" | "announcement";
  isE2ee?: boolean;
  disappearAfterSeconds?: number;
  /** v0.14 — stream/topic-threaded channel */
  isStream?: boolean;
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
  /** v0.14 */
  forwardedFromMessageId?: string;
  forwardedFromChannelId?: string;
  stickerIds?: string[];
  topic?: string;
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

export interface InAppNotification {
  id: string;
  channelId: string;
  channelName?: string;
  authorId: string;
  authorUsername: string;
  content: string;
  createdAt: string;
  read: boolean;
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

// ── Phase 13 engagement types ────────────────────────────────────────────────

export interface Poll {
  id: string;
  channelId: string;
  messageId?: string;
  authorId: string;
  question: string;
  options: string[];
  endsAt?: string;
  allowMultiselect: boolean;
  isAnonymous: boolean;
  status: "open" | "ended";
  createdAt: string;
  updatedAt: string;
}

export interface PollOptionResult {
  index: number;
  label: string;
  voteCount: number;
  voterIds: string[];
}

export interface PollResults {
  poll: Poll;
  options: PollOptionResult[];
  totalVoters: number;
}

export interface ScheduledMessage {
  id: string;
  channelId: string;
  authorId: string;
  content: string;
  scheduledAt: string;
  status: "pending" | "sent" | "failed" | "cancelled";
  createdAt: string;
}

export interface BookmarkedMessagePreview {
  id: string;
  content: string;
  authorId: string;
  authorUsername: string;
  channelId: string;
  createdAt: string;
}

export interface Bookmark {
  id: string;
  userId: string;
  messageId: string;
  channelId: string;
  note?: string;
  createdAt: string;
  message?: BookmarkedMessagePreview;
}

// ─── v0.14 Platform Differentiation types ────────────────────────────────────

export type ServerEventStatus = "scheduled" | "active" | "completed" | "cancelled";

export interface ServerEvent {
  id: string;
  serverId: string;
  creatorId: string;
  title: string;
  description?: string;
  startsAt: string;
  endsAt?: string;
  location?: string;
  channelId?: string;
  coverImage?: string;
  status: ServerEventStatus;
  interestedCount: number;
  isInterested: boolean;
  createdAt: string;
  updatedAt: string;
}

export type StickerType = "png" | "apng" | "lottie";

export interface Sticker {
  id: string;
  packId: string;
  serverId?: string;
  name: string;
  description?: string;
  assetUrl: string;
  type: StickerType;
  createdAt: string;
}

export interface StickerPack {
  id: string;
  name: string;
  description?: string;
  coverStickerId?: string;
  serverId?: string;
  isPremium: boolean;
  stickers?: Sticker[];
}

export interface InlineSuggestion {
  title: string;
  description?: string;
  content: string;
  previewUrl?: string;
}

// ── v0.15 Community Ecosystem ────────────────────────────────────────────────

export interface UserBadge {
  id: string;
  userId: string;
  badgeType: string;
  serverId?: string;
  awardedBy?: string;
  awardedAt: string;
  label?: string;
  iconUrl?: string;
}

export interface BoosterEntry {
  id: string;
  userId: string;
  serverId: string;
  slot: number;
  startedAt: string;
  expiresAt?: string;
}

export interface BoostTierInfo {
  tier: number;
  boosterCount: number;
  extraEmojiSlots: number;
  uploadLimitBytes: number;
  vanityUrlAvailable: boolean;
  currentVanityCode?: string;
}

export type CanvasBlockType = "heading" | "paragraph" | "image" | "code" | "divider" | "table" | "callout";

export interface CanvasBlock {
  id: string;
  channelId: string;
  blockType: CanvasBlockType;
  content: Record<string, unknown>;
  position: number;
  updatedBy?: string;
  updatedAt: string;
}

// ── Federation types (v0.8.5) ─────────────────────────────────────────────────

export type FederationPolicy = "open" | "closed" | "invite_only";

export interface FederationStatus {
  serverName: string;
  softwareVersion: string;
  federationEnabled: boolean;
  peerCount: number;
  healthyPeerCount: number;
  pendingInboundRequests: number;
  pendingOutboundRequests: number;
  uptimeSeconds: number;
}

export interface FederationIdentity {
  displayName?: string;
  description?: string;
  adminContact?: string;
  federationPolicy: FederationPolicy;
}

export interface FederatedPeer {
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

export interface PeeringRequest {
  id: string;
  direction: "inbound" | "outbound";
  remoteDomain: string;
  remoteDisplayName?: string;
  remoteDescription?: string;
  status: "pending" | "accepted" | "rejected" | "cancelled";
  message?: string;
  createdAt: string;
}

export interface FederationAuditEntry {
  id: string;
  adminId: string;
  action: string;
  targetDomain: string;
  details: Record<string, unknown>;
  createdAt: string;
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

  // In-app notifications (@mentions)
  inAppNotifications: InAppNotification[];
  unreadNotificationCount: number;
  addInAppNotification: (notif: Omit<InAppNotification, 'read'>) => void;
  markNotificationsRead: () => void;
  clearNotifications: () => void;

  // Server members — keyed by serverId
  members: Record<string, ServerMember[]>;
  loadMembers: (serverId: string) => Promise<void>;

  // Polls — keyed by channelId
  channelPolls: Record<string, Poll[]>;
  setChannelPolls: (channelId: string, polls: Poll[]) => void;
  updatePoll: (channelId: string, pollId: string, updater: (p: Poll) => Poll) => void;

  // Poll results — keyed by pollId
  pollResults: Record<string, PollResults>;
  setPollResults: (pollId: string, results: PollResults) => void;

  // Drafts (mirrors localStorage for UI reactivity)
  drafts: Record<string, string>;
  setDraft: (channelId: string, text: string) => void;

  // Bookmarks
  bookmarks: Bookmark[];
  setBookmarks: (bookmarks: Bookmark[]) => void;
  addBookmark: (bookmark: Bookmark) => void;
  removeBookmark: (messageId: string) => void;
  loadBookmarks: () => Promise<void>;

  // Note-to-self / Saved Notes
  noteToSelfChannelId: string | null;
  setNoteToSelfChannelId: (id: string | null) => void;
  loadNoteToSelfChannel: () => Promise<void>;

  // Saved Messages panel visibility
  savedMessagesPanelOpen: boolean;
  setSavedMessagesPanelOpen: (v: boolean) => void;

  // Scheduled messages — keyed by channelId
  channelScheduled: Record<string, ScheduledMessage[]>;
  setChannelScheduled: (channelId: string, msgs: ScheduledMessage[]) => void;

  // ─── v0.14 Platform Differentiation ───────────────────────────────────

  // Message forwarding — the message being forwarded (drives ForwardModal)
  forwardModalMessage: Message | null;
  setForwardModalMessage: (msg: Message | null) => void;

  // Server events — keyed by serverId
  serverEvents: Record<string, ServerEvent[]>;
  setServerEvents: (serverId: string, events: ServerEvent[]) => void;
  upsertServerEvent: (serverId: string, event: ServerEvent) => void;
  removeServerEvent: (serverId: string, eventId: string) => void;
  eventsOpen: boolean;
  setEventsOpen: (open: boolean) => void;
  loadServerEvents: (serverId: string, status?: string) => Promise<void>;

  // Sticker packs (global)
  stickerPacks: StickerPack[];
  setStickerPacks: (packs: StickerPack[]) => void;
  loadStickerPacks: () => Promise<void>;

  // Server stickers — keyed by serverId
  serverStickers: Record<string, Sticker[]>;
  setServerStickers: (serverId: string, stickers: Sticker[]) => void;
  loadServerStickers: (serverId: string) => Promise<void>;

  // ─── v0.15 Community Ecosystem ────────────────────────────────────────

  // User badges — keyed by userId
  userBadges: Record<string, UserBadge[]>;
  setUserBadges: (userId: string, badges: UserBadge[]) => void;
  loadUserBadges: (userId: string) => Promise<void>;

  // Server boost tier — keyed by serverId
  boostTierInfo: Record<string, BoostTierInfo>;
  setBoostTierInfo: (serverId: string, info: BoostTierInfo) => void;
  loadBoostTierInfo: (serverId: string) => Promise<void>;

  // Canvas blocks — keyed by channelId
  canvasBlocks: Record<string, CanvasBlock[]>;
  setCanvasBlocks: (channelId: string, blocks: CanvasBlock[]) => void;
  upsertCanvasBlock: (channelId: string, block: CanvasBlock) => void;
  removeCanvasBlock: (channelId: string, blockId: string) => void;
  loadCanvasBlocks: (channelId: string) => Promise<void>;

  // Federation (v0.8.5) — instance-admin management
  federationStatus: FederationStatus | null;
  loadFederationStatus: () => Promise<void>;
  federationIdentity: FederationIdentity | null;
  loadFederationIdentity: () => Promise<void>;
  federatedPeers: FederatedPeer[];
  loadFederatedPeers: () => Promise<void>;
  peeringRequests: PeeringRequest[];
  loadPeeringRequests: (status?: string) => Promise<void>;
}

// Module-level map so typing-clear timeouts survive re-renders
const _typingTimers = new Map<string, ReturnType<typeof setTimeout>>();

// ── Session persistence ───────────────────────────────────────────────────────
const SESSION_KEY = "nexus:session";

function loadPersistedSession(): Session | null {
  try {
    const raw = localStorage.getItem(SESSION_KEY);
    if (raw) return JSON.parse(raw) as Session;
  } catch { /* corrupt data — ignore */ }
  return null;
}

function persistSession(session: Session | null) {
  if (session) {
    localStorage.setItem(SESSION_KEY, JSON.stringify(session));
  } else {
    localStorage.removeItem(SESSION_KEY);
  }
}

export const useStore = create<StoreState>((set, get) => ({
  // ─── Auth ─────────────────────────────────────────────────────────────
  session: loadPersistedSession(),
  setSession: (session) => {
    persistSession(session);
    set({ session });
  },

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
    persistSession(null);
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
      inAppNotifications: [],
      unreadNotificationCount: 0,
      channelPolls: {},
      pollResults: {},
      drafts: {},
      bookmarks: [],
      noteToSelfChannelId: null,
      savedMessagesPanelOpen: false,
      channelScheduled: {},
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

  // ─── In-app notifications ─────────────────────────────────────────────
  inAppNotifications: [],
  unreadNotificationCount: 0,
  addInAppNotification: (notif) =>
    set((s) => {
      const full: InAppNotification = { ...notif, read: false };
      // Cap at 100 most recent notifications
      const updated = [full, ...s.inAppNotifications].slice(0, 100);
      return {
        inAppNotifications: updated,
        unreadNotificationCount: s.unreadNotificationCount + 1,
      };
    }),
  markNotificationsRead: () =>
    set((s) => ({
      inAppNotifications: s.inAppNotifications.map((n) => ({ ...n, read: true })),
      unreadNotificationCount: 0,
    })),
  clearNotifications: () =>
    set({ inAppNotifications: [], unreadNotificationCount: 0 }),

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

  // ─── Polls ────────────────────────────────────────────────────────────
  channelPolls: {},
  setChannelPolls: (channelId, polls) =>
    set((s) => ({ channelPolls: { ...s.channelPolls, [channelId]: polls } })),
  updatePoll: (channelId, pollId, updater) =>
    set((s) => {
      const existing = s.channelPolls[channelId];
      if (!existing) return s;
      return {
        channelPolls: {
          ...s.channelPolls,
          [channelId]: existing.map((p) => (p.id === pollId ? updater(p) : p)),
        },
      };
    }),
  pollResults: {},
  setPollResults: (pollId, results) =>
    set((s) => ({ pollResults: { ...s.pollResults, [pollId]: results } })),

  // ─── Drafts ───────────────────────────────────────────────────────────
  drafts: (() => {
    // Re-hydrate from localStorage on first load
    const map: Record<string, string> = {};
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith("nexus:draft:")) {
        const channelId = key.slice("nexus:draft:".length);
        map[channelId] = localStorage.getItem(key) ?? "";
      }
    }
    return map;
  })(),
  setDraft: (channelId, text) => {
    if (text) {
      localStorage.setItem(`nexus:draft:${channelId}`, text);
    } else {
      localStorage.removeItem(`nexus:draft:${channelId}`);
    }
    set((s) => ({ drafts: { ...s.drafts, [channelId]: text } }));
  },

  // ─── Bookmarks ────────────────────────────────────────────────────────
  bookmarks: [],
  setBookmarks: (bookmarks) => set({ bookmarks }),
  addBookmark: (bookmark) =>
    set((s) => {
      if (s.bookmarks.some((b) => b.messageId === bookmark.messageId)) return s;
      return { bookmarks: [bookmark, ...s.bookmarks] };
    }),
  removeBookmark: (messageId) =>
    set((s) => ({ bookmarks: s.bookmarks.filter((b) => b.messageId !== messageId) })),
  loadBookmarks: async () => {
    try {
      const raw = await invoke<Bookmark[]>("list_bookmarks");
      set({ bookmarks: raw });
    } catch (e) {
      console.error("loadBookmarks error", e);
    }
  },

  // ─── Note-to-self ─────────────────────────────────────────────────────
  noteToSelfChannelId: null,
  setNoteToSelfChannelId: (id) => set({ noteToSelfChannelId: id }),
  loadNoteToSelfChannel: async () => {
    try {
      const res = await invoke<{ channelId: string }>("get_note_to_self_channel");
      set({ noteToSelfChannelId: res.channelId });
    } catch (e) {
      console.error("loadNoteToSelfChannel error", e);
    }
  },

  // ─── Saved Messages panel ─────────────────────────────────────────────
  savedMessagesPanelOpen: false,
  setSavedMessagesPanelOpen: (v) => set({ savedMessagesPanelOpen: v }),

  // ─── Scheduled messages ───────────────────────────────────────────────
  channelScheduled: {},
  setChannelScheduled: (channelId, msgs) =>
    set((s) => ({ channelScheduled: { ...s.channelScheduled, [channelId]: msgs } })),

  // ─── v0.14 Platform Differentiation ──────────────────────────────────
  forwardModalMessage: null,
  setForwardModalMessage: (msg) => set({ forwardModalMessage: msg }),

  serverEvents: {},
  setServerEvents: (serverId, events) =>
    set((s) => ({ serverEvents: { ...s.serverEvents, [serverId]: events } })),
  upsertServerEvent: (serverId, event) =>
    set((s) => {
      const existing = s.serverEvents[serverId] ?? [];
      const without = existing.filter((e) => e.id !== event.id);
      return { serverEvents: { ...s.serverEvents, [serverId]: [event, ...without] } };
    }),
  removeServerEvent: (serverId, eventId) =>
    set((s) => ({
      serverEvents: {
        ...s.serverEvents,
        [serverId]: (s.serverEvents[serverId] ?? []).filter((e) => e.id !== eventId),
      },
    })),
  eventsOpen: false,
  setEventsOpen: (open) => set({ eventsOpen: open }),
  loadServerEvents: async (serverId, status) => {
    try {
      const events = await invoke<ServerEvent[]>("list_server_events", { serverId, status });
      set((s) => ({ serverEvents: { ...s.serverEvents, [serverId]: events } }));
    } catch (e) {
      console.error("loadServerEvents error", e);
    }
  },

  stickerPacks: [],
  setStickerPacks: (packs) => set({ stickerPacks: packs }),
  loadStickerPacks: async () => {
    try {
      const packs = await invoke<StickerPack[]>("list_sticker_packs");
      set({ stickerPacks: packs });
    } catch (e) {
      console.error("loadStickerPacks error", e);
    }
  },

  serverStickers: {},
  setServerStickers: (serverId, stickers) =>
    set((s) => ({ serverStickers: { ...s.serverStickers, [serverId]: stickers } })),
  loadServerStickers: async (serverId) => {
    try {
      const stickers = await invoke<Sticker[]>("list_server_stickers", { serverId });
      set((s) => ({ serverStickers: { ...s.serverStickers, [serverId]: stickers } }));
    } catch (e) {
      console.error("loadServerStickers error", e);
    }
  },

  // ─── v0.15 Community Ecosystem ────────────────────────────────────────

  userBadges: {},
  setUserBadges: (userId, badges) =>
    set((s) => ({ userBadges: { ...s.userBadges, [userId]: badges } })),
  loadUserBadges: async (userId) => {
    try {
      const badges = await invoke<UserBadge[]>("list_user_badges", { userId });
      set((s) => ({ userBadges: { ...s.userBadges, [userId]: badges } }));
    } catch (e) {
      console.error("loadUserBadges error", e);
    }
  },

  boostTierInfo: {},
  setBoostTierInfo: (serverId, info) =>
    set((s) => ({ boostTierInfo: { ...s.boostTierInfo, [serverId]: info } })),
  loadBoostTierInfo: async (serverId) => {
    try {
      const info = await invoke<BoostTierInfo>("get_server_boost_tier", { serverId });
      set((s) => ({ boostTierInfo: { ...s.boostTierInfo, [serverId]: info } }));
    } catch (e) {
      console.error("loadBoostTierInfo error", e);
    }
  },

  canvasBlocks: {},
  setCanvasBlocks: (channelId, blocks) =>
    set((s) => ({ canvasBlocks: { ...s.canvasBlocks, [channelId]: blocks } })),
  upsertCanvasBlock: (channelId, block) =>
    set((s) => {
      const existing = s.canvasBlocks[channelId] ?? [];
      const without = existing.filter((b) => b.id !== block.id);
      const updated = [...without, block].sort((a, b) => a.position - b.position);
      return { canvasBlocks: { ...s.canvasBlocks, [channelId]: updated } };
    }),
  removeCanvasBlock: (channelId, blockId) =>
    set((s) => ({
      canvasBlocks: {
        ...s.canvasBlocks,
        [channelId]: (s.canvasBlocks[channelId] ?? []).filter((b) => b.id !== blockId),
      },
    })),
  loadCanvasBlocks: async (channelId) => {
    try {
      const blocks = await invoke<CanvasBlock[]>("get_canvas", { channelId });
      set((s) => ({ canvasBlocks: { ...s.canvasBlocks, [channelId]: blocks } }));
    } catch (e) {
      console.error("loadCanvasBlocks error", e);
    }
  },

  // ─── Federation ───────────────────────────────────────────────────────
  federationStatus: null,
  loadFederationStatus: async () => {
    try {
      const data = await invoke<FederationStatus>("get_federation_status", {});
      set({ federationStatus: data });
    } catch (e) {
      console.error("loadFederationStatus error", e);
    }
  },

  federationIdentity: null,
  loadFederationIdentity: async () => {
    try {
      const data = await invoke<FederationIdentity>("get_federation_identity", {});
      set({ federationIdentity: data });
    } catch (e) {
      console.error("loadFederationIdentity error", e);
    }
  },

  federatedPeers: [],
  loadFederatedPeers: async () => {
    try {
      const data = await invoke<{ peers: FederatedPeer[] }>("list_peers", {});
      set({ federatedPeers: data.peers ?? [] });
    } catch (e) {
      console.error("loadFederatedPeers error", e);
    }
  },

  peeringRequests: [],
  loadPeeringRequests: async (status?) => {
    try {
      const data = await invoke<{ requests: PeeringRequest[] }>(
        "list_peering_requests", { status }
      );
      set({ peeringRequests: data.requests ?? [] });
    } catch (e) {
      console.error("loadPeeringRequests error", e);
    }
  },
}));

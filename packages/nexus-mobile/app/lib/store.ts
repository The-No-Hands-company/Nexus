/**
 * store.ts - Zustand-compatible React Native store with AsyncStorage persistence
 */
import AsyncStorage from "@react-native-async-storage/async-storage";
import { api, getDefaultServerOrigin, normalizeApiBaseUrl } from "./api";
import type { User, Server, Channel, Message, DmChannel, Relationship, ServerMember } from "./api";

export { api };

const STORAGE_KEY_SESSION = "nexus_session";
const STORAGE_KEY_SETTINGS = "nexus_settings";
const STORAGE_KEY_SERVER_URL = "nexus_server_url";

export interface Session {
  user: User;
  accessToken: string;
  refreshToken: string;
}

type Listener = () => void;

function initialServerOrigin(): string {
  return getDefaultServerOrigin() || "";
}

class Store {
  session: Session | null = null;
  serverUrl = initialServerOrigin();
  servers: Server[] = [];
  activeServerId: string | null = null;
  channels: Channel[] = [];
  activeChannelId: string | null = null;
  messages: Record<string, Message[]> = {};
  dmChannels: DmChannel[] = [];
  relationships: Relationship[] = [];
  members: Record<string, ServerMember[]> = {};
  typingUsers: Record<string, string[]> = {};
  unreadChannels: Record<string, boolean> = {};
  voiceJoinedChannelId: string | null = null;
  voiceMuted = false;
  voiceDeafened = false;
  experienceMode: "full" | "messaging" = "full";
  settings = {
    theme: "dark",
    fontSize: 16,
    compactMode: false,
    reducedMotion: false,
    notificationsEnabled: true,
    mentionsOnly: false,
    soundsEnabled: true,
    vibrationEnabled: true,
    readReceipts: true,
    sharePresence: true,
    dmPermission: "everyone" as "everyone" | "friends" | "nobody",
  };
  updateSettings = (patch: Partial<Store["settings"]>) => {
    this.settings = { ...this.settings, ...patch };
    this._persistSettings();
    this.emit();
  };
  loading = false;
  error: string | null = null;

  get api() { return api; }

  private listeners = new Set<Listener>();
  subscribe(fn: Listener) { this.listeners.add(fn); return () => { this.listeners.delete(fn); }; }
  addListener = (fn: Listener) => { this.listeners.add(fn); return () => { this.listeners.delete(fn); }; };
  private emit() { this.listeners.forEach(fn => fn()); }
  notify() { this.emit(); }

  get hasSession() { return !!this.session; }
  get isAuthenticated() { return !!this.session; }

  // ── Persistence ──────────────────────────────────────────────────────────────

  /** Load session + settings from AsyncStorage. Call once at app startup. */
  async hydrate(): Promise<void> {
    try {
      const [sessionJson, settingsJson, serverUrlJson] = await Promise.all([
        AsyncStorage.getItem(STORAGE_KEY_SESSION),
        AsyncStorage.getItem(STORAGE_KEY_SETTINGS),
        AsyncStorage.getItem(STORAGE_KEY_SERVER_URL),
      ]);
      if (serverUrlJson) {
        const savedServerUrl = JSON.parse(serverUrlJson);
        if (typeof savedServerUrl === "string" && savedServerUrl.trim()) {
          this.serverUrl = savedServerUrl.trim().replace(/\/$/, "");
        }
      }
      if (this.serverUrl) {
        api.setBaseUrl(this.serverUrl);
      }
      if (sessionJson) {
        const saved: Session = JSON.parse(sessionJson);
        // Restore tokens so the api client can make authenticated requests
        api.setTokens(saved.accessToken, saved.refreshToken);
        this.session = saved;
      }
      if (settingsJson) {
        const saved = JSON.parse(settingsJson);
        this.settings = { ...this.settings, ...saved };
      }
    } catch (e) {
      console.error("store hydrate error", e);
    }
  }

  async setServerUrl(url: string) {
    const normalized = url.trim().replace(/\/$/, "");
    if (!normalized) {
      const fallback = getDefaultServerOrigin();
      if (!fallback) {
        throw new Error("Server URL is required");
      }
      this.serverUrl = fallback;
    } else {
      this.serverUrl = normalizeApiBaseUrl(normalized).replace(/\/api\/v1$/, "");
    }
    api.setBaseUrl(this.serverUrl);
    await AsyncStorage.setItem(STORAGE_KEY_SERVER_URL, JSON.stringify(this.serverUrl)).catch(
      e => console.error("persistServerUrl error", e),
    );
    this.emit();
  }

  private async _persistSession(): Promise<void> {
    if (this.session) {
      await AsyncStorage.setItem(STORAGE_KEY_SESSION, JSON.stringify(this.session)).catch(
        e => console.error("persistSession error", e),
      );
    } else {
      await AsyncStorage.removeItem(STORAGE_KEY_SESSION).catch(
        e => console.error("clearSession error", e),
      );
    }
  }

  private _persistSettings(): void {
    AsyncStorage.setItem(STORAGE_KEY_SETTINGS, JSON.stringify(this.settings)).catch(
      e => console.error("persistSettings error", e),
    );
  }

  // ── Auth ─────────────────────────────────────────────────────────────────────

  async login(username: string, password: string) {
    this.loading = true; this.error = null; this.emit();
    try {
      const r = await api.login(username, password);
      this.session = { user: r.user, accessToken: r.accessToken, refreshToken: r.refreshToken };
      await this._persistSession();
      await this.loadInitialData();
    } catch (e: unknown) {
      this.error = e instanceof Error ? e.message : "Login failed";
    } finally {
      this.loading = false; this.emit();
    }
  }

  async register(username: string, password: string, email?: string) {
    this.loading = true; this.error = null; this.emit();
    try {
      const r = await api.register(username, password, email);
      this.session = { user: r.user, accessToken: r.accessToken, refreshToken: r.refreshToken };
      await this._persistSession();
      await this.loadInitialData();
    } catch (e: unknown) {
      this.error = e instanceof Error ? e.message : "Registration failed";
    } finally {
      this.loading = false; this.emit();
    }
  }

  logout() {
    api.logout();
    this.session = null;
    this.servers = []; this.activeServerId = null;
    this.channels = []; this.activeChannelId = null;
    this.messages = {}; this.dmChannels = [];
    this.relationships = []; this.members = {};
    this._persistSession();
    this.emit();
  }

  async loadInitialData() {
    try {
      const [servers, dms, relationships] = await Promise.all([
        api.getServers(), api.getDMs(), api.getRelationships(),
      ]);
      this.servers = servers; this.dmChannels = dms; this.relationships = relationships;
      this.emit();
    } catch (e) { console.error("loadInitialData error", e); }
  }

  async sendTyping(channelId: string) {
    try {
      await api.sendTyping(channelId);
    } catch (e) {
      console.error("sendTyping error", e);
    }
  }

  async selectServer(serverId: string) {
    this.activeServerId = serverId; this.channels = []; this.activeChannelId = null;
    this.emit();
    try {
      const channels = await api.getChannels(serverId);
      this.channels = channels;
      const first = channels.find(c => c.kind === "text" || c.kind === "announcement" || c.kind === "forum");
      if (first) await this.selectChannel(first.id);
      this.emit();
    } catch (e) { console.error("selectServer error", e); }
  }

  async createServer(name: string) {
    const server = await api.createServer(name);
    this.servers = [...this.servers, server]; this.emit();
    return server;
  }

  async loadMembers(serverId: string) {
    try {
      const members = await api.getMembers(serverId);
      this.members = { ...this.members, [serverId]: members }; this.emit();
    } catch (e) { console.error("loadMembers error", e); }
  }

  async selectChannel(channelId: string) {
    this.activeChannelId = channelId;
    const unread = { ...this.unreadChannels };
    delete unread[channelId];
    this.unreadChannels = unread;
    this.emit();
    await this.loadMessages(channelId);
  }

  async loadMessages(channelId: string, before?: string) {
    try {
      const msgs = await api.getMessages(channelId, before);
      if (before) {
        const existing = this.messages[channelId] ?? [];
        this.messages = { ...this.messages, [channelId]: [...existing, ...msgs] };
      } else {
        this.messages = { ...this.messages, [channelId]: msgs };
      }
      this.emit();
    } catch (e) { console.error("loadMessages error", e); }
  }

  async loadOlderMessages() {
    if (!this.activeChannelId) return;
    const msgs = this.messages[this.activeChannelId];
    if (!msgs?.length) return;
    await this.loadMessages(this.activeChannelId, msgs[0].id);
  }

  async sendMessage(content: string, replyTo?: string) {
    if (!this.activeChannelId) return;
    const optimistic: Message = {
      id: "temp-" + Date.now(), channelId: this.activeChannelId,
      authorId: this.session!.user.id, authorUsername: this.session!.user.username,
      authorAvatar: this.session!.user.avatar, content, createdAt: new Date().toISOString(), replyTo,
    };
    const msgs = [optimistic, ...(this.messages[this.activeChannelId] ?? [])];
    this.messages = { ...this.messages, [this.activeChannelId]: msgs };
    this.emit();
    try {
      const sent = await api.sendMessage(this.activeChannelId, content, replyTo);
      const updated = this.messages[this.activeChannelId].map(m => m.id === optimistic.id ? sent : m);
      this.messages = { ...this.messages, [this.activeChannelId]: updated };
      this.emit();
    } catch {
      this.messages = {
        ...this.messages,
        [this.activeChannelId]: this.messages[this.activeChannelId].filter(m => m.id !== optimistic.id),
      };
      this.emit();
    }
  }

  appendMessage(msg: Message) {
    const existing = this.messages[msg.channelId] ?? [];
    if (existing.some(m => m.id === msg.id)) return;
    const msgs = [msg, ...existing];
    this.messages = { ...this.messages, [msg.channelId]: msgs };
    const unread = { ...this.unreadChannels };
    if (this.activeChannelId !== msg.channelId) unread[msg.channelId] = true;
    this.unreadChannels = unread;
    this.emit();
  }

  prependOlderMessages(channelId: string, msgs: Message[]) {
    const existing = this.messages[channelId] ?? [];
    this.messages = { ...this.messages, [channelId]: [...existing, ...msgs] };
    this.emit();
  }

  async addReaction(channelId: string, messageId: string, emoji: string) {
    await api.addReaction(channelId, messageId, emoji);
    this.toggleReaction(channelId, messageId, emoji, true);
  }

  async removeReaction(channelId: string, messageId: string, emoji: string) {
    await api.removeReaction(channelId, messageId, emoji);
    this.toggleReaction(channelId, messageId, emoji, false);
  }

  private toggleReaction(channelId: string, messageId: string, emoji: string, add: boolean) {
    const msgs = this.messages[channelId];
    if (!msgs) return;
    this.messages = {
      ...this.messages,
      [channelId]: msgs.map(m => {
        if (m.id !== messageId) return m;
        const reactions = [...(m.reactions ?? [])];
        const idx = reactions.findIndex(r => r.emoji === emoji);
        if (add) {
          if (idx >= 0) { reactions[idx] = { ...reactions[idx], count: reactions[idx].count + 1, me: true }; }
          else { reactions.push({ emoji, count: 1, me: true }); }
        } else {
          if (idx >= 0) {
            const updated = { ...reactions[idx], count: reactions[idx].count - 1 };
            if (updated.count <= 0) reactions.splice(idx, 1);
            else reactions[idx] = updated;
          }
        }
        return { ...m, reactions };
      }),
    };
    this.emit();
  }

  async createChannel(serverId: string, name: string, kind: Channel["kind"] = "text") {
    const channel = await api.createChannel(serverId, { name, kind });
    this.channels = [...this.channels, channel]; this.emit();
    return channel;
  }

  async selectDM(channelId: string) {
    this.activeChannelId = channelId;
    this.activeServerId = null;
    const unread = { ...this.unreadChannels };
    delete unread[channelId];
    this.unreadChannels = unread;
    this.emit();
    await this.loadMessages(channelId);
  }

  async createDM(recipientId: string) {
    const dm = await api.createDM(recipientId);
    this.dmChannels = [...this.dmChannels.filter(d => d.id !== dm.id), dm]; this.emit();
    return dm;
  }

  async joinVoice(channelId: string) {
    this.voiceJoinedChannelId = channelId;
    this.emit();
    try {
      await api.joinVoice(channelId);
    } catch (e) { console.error("joinVoice error", e); }
  }

  async leaveVoice() {
    if (!this.voiceJoinedChannelId) return;
    const channelId = this.voiceJoinedChannelId;
    this.voiceJoinedChannelId = null;
    this.voiceMuted = false;
    this.voiceDeafened = false;
    this.emit();
    try {
      await api.leaveVoice(channelId);
    } catch (e) { console.error("leaveVoice error", e); }
  }

  async toggleMute() {
    this.voiceMuted = !this.voiceMuted;
    this.emit();
    try {
      await api.toggleMute();
    } catch (e) { console.error("toggleMute error", e); }
  }

  async toggleDeafen() {
    this.voiceDeafened = !this.voiceDeafened;
    this.emit();
    try {
      await api.toggleDeafen();
    } catch (e) { console.error("toggleDeafen error", e); }
  }
}

export const store = new Store();

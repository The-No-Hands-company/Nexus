/**
 * api.ts - REST API client for Nexus Mobile
 */

export const DEFAULT_API_BASE = "http://localhost:8080/api/v1";

// ── Types ─────────────────────────────────────────────────────────────────────

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
}

export interface User {
  id: string;
  username: string;
  displayName?: string;
  avatar?: string;
  email?: string;
  presence?: string;
  status?: string;
  bio?: string;
  createdAt: string;
}

export interface Server {
  id: string;
  name: string;
  icon?: string;
  ownerId: string;
  memberCount?: number;
  description?: string;
  isPublic?: boolean;
}

export interface Channel {
  id: string;
  serverId?: string;
  name: string;
  kind: "text" | "voice" | "announcement" | "forum" | "stage" | "dm" | "group_dm";
  topic?: string;
  isE2ee?: boolean;
  disappearAfterSeconds?: number;
  isStream?: boolean;
}

export interface Message {
  id: string;
  channelId: string;
  authorId: string;
  authorUsername: string;
  authorAvatar?: string;
  content: string;
  createdAt: string;
  editedAt?: string;
  replyTo?: string;
  threadId?: string;
  attachments?: Attachment[];
  reactions?: Reaction[];
  embeds?: Embed[];
  stickerIds?: string[];
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

export interface Relationship {
  id: string;
  direction: "incoming" | "outgoing";
  status: "pending" | "accepted" | "blocked" | "denied";
  user: { id: string; username: string; displayName?: string; avatar?: string };
}

export interface DmChannel {
  id: string;
  channelType: "dm" | "group_dm";
  name?: string;
  recipients: { id: string; username: string; displayName?: string; avatar?: string }[];
  lastMessageId?: string;
}

export interface ServerMember {
  userId: string;
  username: string;
  displayName?: string;
  avatar?: string;
  nickname?: string;
  presence: "online" | "idle" | "do_not_disturb" | "invisible" | "offline";
  roles: string[];
}

// ── API client ────────────────────────────────────────────────────────────────

class NexusApi {
  private baseUrl: string;
  private _accessToken: string | null = null;
  private refreshToken: string | null = null;

  constructor(baseUrl: string = DEFAULT_API_BASE) {
    this.baseUrl = baseUrl;
  }

  // ── Config ──────────────────────────────────────────────────────────────────

  getBaseUrl(): string { return this.baseUrl; }
  setBaseUrl(url: string): void { this.baseUrl = url; }

  setTokens(access: string, refresh: string) {
    this._accessToken = access;
    this.refreshToken = refresh;
  }

  clearTokens() {
    this._accessToken = null;
    this.refreshToken = null;
  }

  get hasToken(): boolean { return !!this._accessToken; }

  // ── Core ────────────────────────────────────────────────────────────────────

  private headers(): Record<string, string> {
    const h: Record<string, string> = { "Content-Type": "application/json" };
    if (this._accessToken) h["Authorization"] = "Bearer " + this._accessToken;
    return h;
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const opts: RequestInit = { method, headers: this.headers() };
    if (body !== undefined) opts.body = JSON.stringify(body);
    const res = await fetch(this.baseUrl + path, opts);
    if (res.status === 401 && this.refreshToken) {
      const ok = await this.tryRefresh();
      if (ok) {
        opts.headers = this.headers();
        const r2 = await fetch(this.baseUrl + path, opts);
        if (r2.ok) return r2.json() as Promise<T>;
      }
      this.clearTokens();
      throw new Error("Unauthorized");
    }
    if (!res.ok) {
      const txt = await res.text();
      throw new Error(res.status + ": " + txt);
    }
    if (res.status === 204) return undefined as unknown as T;
    return res.json() as Promise<T>;
  }

  private async tryRefresh(): Promise<boolean> {
    if (!this.refreshToken) return false;
    try {
      const r = await fetch(this.baseUrl + "/auth/refresh", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ refresh_token: this.refreshToken }),
      });
      if (!r.ok) return false;
      const data = await r.json() as Record<string, unknown>;
      if (typeof data.access_token !== "string") return false;
      this._accessToken = data.access_token as string;
      if (typeof data.refresh_token === "string") {
        this.refreshToken = data.refresh_token as string;
      }
      return true;
    } catch { return false; }
  }

  // ── Auth ────────────────────────────────────────────────────────────────────

  async register(username: string, password: string, email?: string) {
    const r = await this.request<{ user: User } & AuthTokens>("POST", "/auth/register", {
      username, password, ...(email ? { email } : {}),
    });
    this.setTokens(r.accessToken, r.refreshToken);
    return r;
  }

  async login(username: string, password: string) {
    const r = await this.request<{ user: User } & AuthTokens>("POST", "/auth/login", { username, password });
    this.setTokens(r.accessToken, r.refreshToken);
    return r;
  }

  async logout() {
    try { await this.request("POST", "/auth/logout"); } catch { /* ignore */ }
    this.clearTokens();
  }

  async changePassword(currentPassword: string, newPassword: string): Promise<void> {
    await this.request("POST", "/users/@me/change-password", {
      current_password: currentPassword,
      new_password: newPassword,
    });
  }

  // ── Users ───────────────────────────────────────────────────────────────────

  async getMe(): Promise<User> { return this.request("GET", "/users/@me"); }
  async getUser(userId: string): Promise<User> { return this.request("GET", "/users/" + userId); }
  async updateMe(data: Partial<User>): Promise<User> { return this.request("PATCH", "/users/@me", data); }
  async searchUsers(query: string): Promise<User[]> {
    return this.request("GET", "/users/search?q=" + encodeURIComponent(query));
  }

  // ── Servers ─────────────────────────────────────────────────────────────────

  async getServers(): Promise<Server[]> { return this.request("GET", "/users/@me/servers"); }
  async getServer(serverId: string): Promise<Server> { return this.request("GET", "/servers/" + serverId); }
  async createServer(name: string, isPublic = false): Promise<Server> {
    return this.request("POST", "/servers", { name, is_public: isPublic });
  }
  async deleteServer(serverId: string) { return this.request("DELETE", "/servers/" + serverId); }
  async leaveServer(serverId: string) { return this.request("DELETE", "/servers/" + serverId + "/members/@me"); }

  // ── Channels ────────────────────────────────────────────────────────────────

  async getChannels(serverId: string): Promise<Channel[]> {
    return this.request("GET", "/servers/" + serverId + "/channels");
  }
  async getChannel(channelId: string): Promise<Channel> { return this.request("GET", "/channels/" + channelId); }
  async createChannel(serverId: string, data: Partial<Channel>): Promise<Channel> {
    return this.request("POST", "/servers/" + serverId + "/channels", data);
  }
  async updateChannel(channelId: string, data: Partial<Channel>): Promise<Channel> {
    return this.request("PATCH", "/channels/" + channelId, data);
  }
  async deleteChannel(channelId: string) { return this.request("DELETE", "/channels/" + channelId); }

  async sendTyping(channelId: string): Promise<void> {
    this.request("POST", "/channels/" + channelId + "/typing").catch(() => {});
  }

  // ── Messages ─────────────────────────────────────────────────────────────────

  async getMessages(channelId: string, before?: string, limit = 50): Promise<Message[]> {
    let path = "/channels/" + channelId + "/messages?limit=" + limit;
    if (before) path += "&before=" + before;
    return this.request("GET", path);
  }
  async sendMessage(channelId: string, content: string, replyTo?: string, threadId?: string): Promise<Message> {
    return this.request("POST", "/channels/" + channelId + "/messages", {
      content,
      ...(replyTo ? { reply_to: replyTo } : {}),
      ...(threadId ? { thread_id: threadId } : {}),
    });
  }
  async editMessage(messageId: string, content: string): Promise<Message> {
    return this.request("PATCH", "/messages/" + messageId, { content });
  }
  async deleteMessage(messageId: string) { return this.request("DELETE", "/messages/" + messageId); }
  async addReaction(channelId: string, messageId: string, emoji: string) {
    return this.request("PUT",
      "/channels/" + channelId + "/messages/" + messageId + "/reactions/" + encodeURIComponent(emoji));
  }
  async removeReaction(channelId: string, messageId: string, emoji: string) {
    return this.request("DELETE",
      "/channels/" + channelId + "/messages/" + messageId + "/reactions/" + encodeURIComponent(emoji));
  }

  // ── DMs ──────────────────────────────────────────────────────────────────────

  async getDMs(): Promise<DmChannel[]> { return this.request("GET", "/users/@me/channels"); }
  async createDM(recipientId: string): Promise<DmChannel> {
    return this.request("POST", "/users/@me/channels", { recipient_id: recipientId });
  }

  // ── Relationships ─────────────────────────────────────────────────────────────

  async getRelationships(): Promise<Relationship[]> { return this.request("GET", "/users/@me/relationships"); }
  async addFriend(userId: string) { return this.request("POST", "/users/@me/relationships", { user_id: userId, type: 1 }); }
  async blockUser(userId: string) { return this.request("POST", "/users/@me/relationships", { user_id: userId, type: 2 }); }
  async removeRelationship(userId: string) { return this.request("DELETE", "/users/@me/relationships/" + userId); }

  // ── Members ───────────────────────────────────────────────────────────────────

  async getMembers(serverId: string, limit = 100): Promise<ServerMember[]> {
    return this.request("GET", "/servers/" + serverId + "/members?limit=" + limit);
  }

  // ── Invites ───────────────────────────────────────────────────────────────────

  async getInvites(serverId: string) {
    return this.request("GET", "/servers/" + serverId + "/invites");
  }
  async createInvite(serverId: string, maxAge?: number, maxUses?: number) {
    return this.request("POST", "/servers/" + serverId + "/invites", {
      ...(maxAge ? { max_age: maxAge } : {}),
      ...(maxUses ? { max_uses: maxUses } : {}),
    });
  }
  async joinServer(inviteCode: string): Promise<Server> {
    return this.request("POST", "/invites/" + inviteCode + "/join");
  }

  // ── Gateway ───────────────────────────────────────────────────────────────────

  async getGatewayUrl(): Promise<string> {
    const r = await this.request<{ url: string }>("GET", "/gateway");
    return r.url;
  }
}

export const api = new NexusApi();
export { NexusApi };

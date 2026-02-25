/**
 * invoke.ts — browser-compatible shim for @tauri-apps/api/core invoke.
 *
 * When running inside a Tauri webview: delegates to the real Tauri invoke.
 * When running in a plain browser (Vite `npm run dev`): makes fetch requests
 * directly to the Nexus REST API, mapping each command to the equivalent
 * HTTP call and normalising snake_case → camelCase to match Tauri's output.
 */

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// ── Browser-mode session ─────────────────────────────────────────────────────
// In Tauri mode the session lives inside Rust AppState; in the browser we keep
// it here and mirror it in localStorage so it survives a page refresh.

let _serverUrl: string =
  localStorage.getItem("nexus:dev:serverUrl") ?? "http://localhost:8080";
let _token: string | null = localStorage.getItem("nexus:dev:token");
let _refreshToken: string | null = localStorage.getItem("nexus:dev:refreshToken");

function authHeaders(): Record<string, string> {
  const h: Record<string, string> = { "Content-Type": "application/json" };
  if (_token) h["Authorization"] = `Bearer ${_token}`;
  return h;
}

/** Try to exchange the stored refresh token for a new access token. */
async function tryRefreshToken(): Promise<boolean> {
  if (!_refreshToken) return false;
  try {
    const r = await fetch(`${_serverUrl}/api/v1/auth/refresh`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refresh_token: _refreshToken }),
    });
    if (!r.ok) return false;
    const data = await r.json() as Record<string, unknown>;
    if (typeof data.access_token !== "string") return false;
    _token = data.access_token;
    localStorage.setItem("nexus:dev:token", _token);
    if (typeof data.refresh_token === "string") {
      _refreshToken = data.refresh_token;
      localStorage.setItem("nexus:dev:refreshToken", _refreshToken);
    }
    return true;
  } catch {
    return false;
  }
}

async function apiFetch<T>(
  method: string,
  path: string,
  body?: unknown
): Promise<T> {
  const r = await fetch(`${_serverUrl}${path}`, {
    method,
    headers: authHeaders(),
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  // Auto-refresh on 401 and retry once
  if (r.status === 401) {
    const refreshed = await tryRefreshToken();
    if (refreshed) {
      const r2 = await fetch(`${_serverUrl}${path}`, {
        method,
        headers: authHeaders(),
        body: body !== undefined ? JSON.stringify(body) : undefined,
      });
      if (r2.ok) return r2.json() as Promise<T>;
      // Refresh didn't help — clear session
    }
    // Token dead and refresh failed → wipe stored credentials
    _token = null;
    _refreshToken = null;
    localStorage.removeItem("nexus:dev:token");
    localStorage.removeItem("nexus:dev:refreshToken");
    const text = await r.text();
    throw new Error(`401: ${text}`);
  }

  if (!r.ok) {
    const text = await r.text();
    throw new Error(`${r.status}: ${text}`);
  }
  return r.json() as Promise<T>;
}

// ── Shape mappers ─────────────────────────────────────────────────────────────
// The Tauri Rust commands use #[serde(rename_all = "camelCase")] when
// serialising responses back to TypeScript.  Direct API responses are
// snake_case, so we normalise them here.

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Raw = Record<string, any>;

function mapServer(s: Raw) {
  return {
    id: s.id,
    name: s.name,
    icon: s.icon ?? null,
    memberCount: s.member_count ?? null,
    ownerId: s.owner_id,
  };
}

function mapChannel(c: Raw) {
  return {
    id: c.id,
    serverId: c.server_id ?? null,
    name: (c.name as string) ?? "",
    kind: c.channel_type,
    isE2ee: c.encrypted ?? false,
  };
}

function mapMessage(m: Raw) {
  return {
    id: m.id,
    channelId: m.channel_id,
    authorId: m.author_id,
    authorUsername: (m.author_username as string) ?? "Unknown",
    content: m.content,
    createdAt: m.created_at,
    editedAt: m.edited_at ?? null,
    reactions: (m.reactions as Raw[] | undefined)?.map((r) => ({
      emoji: r.emoji as string,
      count: r.count as number,
      me: (r.me as boolean) ?? false,
    })) ?? [],
    embeds: (m.embeds as Raw[] | undefined) ?? [],
    threadId: (m.thread_id as string | undefined) ?? undefined,
  };
}

// ── Command dispatch ──────────────────────────────────────────────────────────

async function browserInvoke<T>(cmd: string, args: Raw = {}): Promise<T> {
  switch (cmd) {
    // ── Auth ──────────────────────────────────────────────────────────────
    case "set_server_url": {
      _serverUrl = args.url as string;
      localStorage.setItem("nexus:dev:serverUrl", _serverUrl);
      return undefined as unknown as T;
    }

    case "login": {
      const resp = await apiFetch<Raw>("POST", "/api/v1/auth/login", {
        username: args.username,
        password: args.password,
      });
      _token = resp.access_token as string;
      localStorage.setItem("nexus:dev:token", _token);
      if (typeof resp.refresh_token === "string") {
        _refreshToken = resp.refresh_token;
        localStorage.setItem("nexus:dev:refreshToken", _refreshToken);
      }
      return resp as T;
    }

    case "register": {
      const resp = await apiFetch<Raw>("POST", "/api/v1/auth/register", {
        username: args.username,
        email: args.email,
        password: args.password,
      });
      _token = resp.access_token as string;
      localStorage.setItem("nexus:dev:token", _token);
      if (typeof resp.refresh_token === "string") {
        _refreshToken = resp.refresh_token;
        localStorage.setItem("nexus:dev:refreshToken", _refreshToken);
      }
      return resp as T;
    }

    case "logout": {
      _token = null;
      _refreshToken = null;
      localStorage.removeItem("nexus:dev:token");
      localStorage.removeItem("nexus:dev:refreshToken");
      return undefined as unknown as T;
    }

    // ── Servers ───────────────────────────────────────────────────────────
    case "list_servers": {
      const raw = await apiFetch<Raw[]>("GET", "/api/v1/servers");
      return raw.map(mapServer) as T;
    }

    case "create_server": {
      const raw = await apiFetch<Raw>("POST", "/api/v1/servers", {
        name: args.name,
        is_public: args.isPublic ?? false,
      });
      return mapServer(raw) as T;
    }

    case "update_server": {
      const raw = await apiFetch<Raw>("PATCH", `/api/v1/servers/${args.serverId}`, {
        name: args.name ?? undefined,
        description: args.description ?? undefined,
        is_public: args.isPublic ?? undefined,
        region: args.region ?? undefined,
      });
      return mapServer(raw) as T;
    }

    case "delete_server": {
      return apiFetch<T>("DELETE", `/api/v1/servers/${args.serverId}`);
    }

    case "list_server_invites": {
      const raw = await apiFetch<Raw[]>("GET", `/api/v1/servers/${args.serverId}/invites`);
      return raw.map((r) => ({
        code: r.code as string,
        serverId: r.server_id as string,
        maxUses: (r.max_uses as number | null) ?? null,
        uses: (r.uses as number) ?? 0,
        expiresAt: (r.expires_at as string | null) ?? null,
        createdAt: r.created_at as string,
      })) as unknown as T;
    }

    case "create_invite": {
      return apiFetch<T>("POST", `/api/v1/servers/${args.serverId}/invites`, {
        max_uses: args.maxUses ?? null,
        max_age_secs: args.maxAgeSecs ?? null,
      });
    }

    case "join_via_invite": {
      return apiFetch<T>("POST", `/api/v1/invites/${args.code}/join`);
    }

    // ── Channels ──────────────────────────────────────────────────────────
    case "list_channels": {
      const raw = await apiFetch<Raw[]>(
        "GET",
        `/api/v1/servers/${args.serverId}/channels`
      );
      return raw.map(mapChannel) as T;
    }

    case "create_channel": {
      const raw = await apiFetch<Raw>(
        "POST",
        `/api/v1/servers/${args.serverId}/channels`,
        { name: args.name, channel_type: args.channelType }
      );
      return mapChannel(raw) as T;
    }

    // ── Messages ──────────────────────────────────────────────────────────
    case "send_message":
    case "send_encrypted_message": {
      const raw = await apiFetch<Raw>(
        "POST",
        `/api/v1/channels/${args.channelId}/messages`,
        { content: args.content }
      );
      return mapMessage(raw) as T;
    }

    case "fetch_history": {
      let url = `/api/v1/channels/${args.channelId}/messages?limit=${
        args.limit ?? 50
      }`;
      if (args.before) url += `&before=${args.before}`;
      const raw = await apiFetch<Raw[]>("GET", url);
      return raw.map(mapMessage) as T;
    }

    case "send_typing": {
      // Fire-and-forget; ignore errors
      apiFetch<void>("POST", `/api/v1/channels/${args.channelId}/typing`).catch(() => {});
      return undefined as unknown as T;
    }

    case "add_reaction": {
      const emoji = encodeURIComponent(String(args.emoji));
      return apiFetch<T>(
        "PUT",
        `/api/v1/channels/${args.channelId}/messages/${args.messageId}/reactions/${emoji}/@me`
      );
    }

    case "remove_reaction": {
      const emoji = encodeURIComponent(String(args.emoji));
      return apiFetch<T>(
        "DELETE",
        `/api/v1/channels/${args.channelId}/messages/${args.messageId}/reactions/${emoji}/@me`
      );
    }

    // ── Threads ───────────────────────────────────────────────────────────
    case "create_thread": {
      const raw = await apiFetch<Raw>("POST", `/api/v1/channels/${args.channelId}/threads`, {
        title: args.title,
        auto_archive_minutes: 60,
      });
      return {
        id: raw.id as string,
        parentChannelId: raw.parent_channel_id as string,
        parentMessageId: (raw.parent_message_id as string | undefined) ?? undefined,
        ownerId: raw.owner_id as string,
        title: raw.title as string,
        messageCount: (raw.message_count as number) ?? 0,
        memberCount: (raw.member_count as number) ?? 0,
        archived: (raw.archived as boolean) ?? false,
        locked: (raw.locked as boolean) ?? false,
        createdAt: raw.created_at as string,
      } as unknown as T;
    }

    case "get_thread": {
      const raw = await apiFetch<Raw>(
        "GET",
        `/api/v1/channels/${args.channelId}/threads/${args.threadId}`
      );
      return {
        id: raw.id as string,
        parentChannelId: raw.parent_channel_id as string,
        parentMessageId: (raw.parent_message_id as string | undefined) ?? undefined,
        ownerId: raw.owner_id as string,
        title: raw.title as string,
        messageCount: (raw.message_count as number) ?? 0,
        memberCount: (raw.member_count as number) ?? 0,
        archived: (raw.archived as boolean) ?? false,
        locked: (raw.locked as boolean) ?? false,
        createdAt: raw.created_at as string,
      } as unknown as T;
    }

    // ── Search ────────────────────────────────────────────────────────────
    case "search_messages": {
      const params = new URLSearchParams({ q: String(args.q) });
      if (args.serverId) params.set("server_id", String(args.serverId));
      if (args.channelId) params.set("channel_id", String(args.channelId));
      if (args.limit) params.set("limit", String(args.limit));
      if (args.offset) params.set("offset", String(args.offset));
      const raw = await apiFetch<Raw>("GET", `/api/v1/search/messages?${params}`);
      return {
        query: raw.query as string,
        totalHits: (raw.total_hits as number | undefined) ?? null,
        hits: ((raw.hits as Raw[]) ?? []).map((h) => ({
          id: h.id as string,
          channelId: h.channel_id as string,
          authorId: h.author_id as string,
          authorUsername: (h.author_username as string) ?? "Unknown",
          content: (h.content as string) ?? "",
          createdAt: h.created_at as string,
        })),
      } as unknown as T;
    }

    case "update_profile": {
      return apiFetch<T>("PATCH", "/api/v1/users/me", {
        display_name: args.displayName ?? undefined,
        avatar_url: args.avatarUrl ?? undefined,
      });
    }

    case "list_devices": {
      const raw = await apiFetch<Raw[]>("GET", "/api/v1/devices");
      return raw.map((d) => ({
        id: d.id as string,
        userId: d.user_id as string,
        name: (d.name as string) ?? "Unknown Device",
        deviceType: (d.device_type as string) ?? "unknown",
        lastSeenAt: (d.last_seen_at as string | undefined) ?? undefined,
        verified: (d.verified as boolean) ?? false,
        createdAt: d.created_at as string,
      })) as unknown as T;
    }

    // ── Friends & Relationships ───────────────────────────────────────
    case "list_relationships": {
      const raw = await apiFetch<Raw[]>("GET", "/api/v1/users/@me/relationships");
      return raw.map((r) => ({
        id: r.id as string,
        direction: r.direction as string,
        status: r.status as string,
        user: {
          id: (r.user as Raw).id as string,
          username: (r.user as Raw).username as string,
          displayName: ((r.user as Raw).display_name as string | undefined) ?? undefined,
          avatar: ((r.user as Raw).avatar as string | undefined) ?? undefined,
        },
      })) as unknown as T;
    }

    case "send_friend_request": {
      const raw = await apiFetch<Raw>("POST", "/api/v1/users/@me/relationships", {
        username: args.username,
      });
      return {
        id: raw.id as string,
        direction: raw.direction as string,
        status: raw.status as string,
        user: {
          id: (raw.user as Raw).id as string,
          username: (raw.user as Raw).username as string,
          displayName: ((raw.user as Raw).display_name as string | undefined) ?? undefined,
          avatar: ((raw.user as Raw).avatar as string | undefined) ?? undefined,
        },
      } as unknown as T;
    }

    case "update_relationship": {
      return apiFetch<T>(
        "PATCH",
        `/api/v1/users/@me/relationships/${String(args.userId)}`,
        { action: args.action }
      );
    }

    case "delete_relationship": {
      return apiFetch<T>(
        "DELETE",
        `/api/v1/users/@me/relationships/${String(args.userId)}`
      );
    }

    case "search_users": {
      const raw = await apiFetch<Raw[]>(
        "GET",
        `/api/v1/users/search?q=${encodeURIComponent(String(args.q))}`
      );
      return raw.map((u) => ({
        id: u.id as string,
        username: u.username as string,
        displayName: (u.display_name as string | undefined) ?? undefined,
        avatar: (u.avatar as string | undefined) ?? undefined,
      })) as unknown as T;
    }

    // ── DM Channels ───────────────────────────────────────────────────
    case "list_dm_channels": {
      const raw = await apiFetch<Raw[]>("GET", "/api/v1/users/@me/channels");
      return raw.map((d) => ({
        id: d.id as string,
        channelType: (d.channel_type as string) as "dm" | "group_dm",
        name: (d.name as string | undefined) ?? undefined,
        lastMessageId: (d.last_message_id as string | undefined) ?? undefined,
        recipients: ((d.recipients as Raw[]) ?? []).map((r) => ({
          id: r.id as string,
          username: r.username as string,
          displayName: (r.display_name as string | undefined) ?? undefined,
          avatar: (r.avatar as string | undefined) ?? undefined,
        })),
      })) as unknown as T;
    }

    case "create_dm": {
      const raw = await apiFetch<Raw>("POST", "/api/v1/users/@me/channels", {
        recipient_id: args.recipientId,
      });
      return {
        id: raw.id as string,
        channelType: (raw.channel_type as string) as "dm" | "group_dm",
        name: (raw.name as string | undefined) ?? undefined,
        lastMessageId: (raw.last_message_id as string | undefined) ?? undefined,
        recipients: ((raw.recipients as Raw[]) ?? []).map((r) => ({
          id: r.id as string,
          username: r.username as string,
          displayName: (r.display_name as string | undefined) ?? undefined,
          avatar: (r.avatar as string | undefined) ?? undefined,
        })),
      } as unknown as T;
    }

    case "list_members": {
      const rows = await apiFetch<Raw[]>("GET", `/api/v1/servers/${args.serverId}/members`);
      return rows as unknown as T;
    }

    case "get_user_profile": {
      return apiFetch<T>("GET", `/api/v1/users/${args.userId}/profile`);
    }

    // ── Desktop-only commands (no-ops in browser) ─────────────────────────
    case "install_update":
      console.info("[browser] install_update is a no-op in the browser");
      return undefined as unknown as T;

    default:
      throw new Error(`[browser] Unhandled invoke command: "${cmd}"`);
  }
}

// ── Public API ────────────────────────────────────────────────────────────────

export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (isTauri()) {
    // Dynamic import so the Tauri module is never bundled when running in browser
    const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
    return tauriInvoke<T>(cmd, args);
  }
  return browserInvoke<T>(cmd, args ?? {});
}

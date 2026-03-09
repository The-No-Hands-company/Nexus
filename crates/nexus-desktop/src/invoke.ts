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
  localStorage.getItem("nexus:lastServerUrl") ?? localStorage.getItem("nexus:dev:serverUrl") ?? "http://localhost:8080";

/** Return the currently configured server URL (e.g. "https://nexus-tnhc.fly.dev"). */
export function getServerUrl(): string {
  return _serverUrl;
}

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
    require2fa: (s.require_2fa as boolean) ?? false,
    spamWindowSecs: (s.spam_window_secs as number) ?? 30,
    spamMaxMessages: (s.spam_max_messages as number) ?? 3,
    boostTier: (s.boost_tier as number) ?? 0,
    boosterCount: (s.booster_count as number) ?? 0,
    vanityCode: s.vanity_code ?? null,
    tags: (s.tags as string[]) ?? [],
    category: s.category ?? null,
    activityScore: (s.activity_score as number) ?? 0,
    featuredAt: s.featured_at ?? null,
    tipJarUrl: s.tip_jar_url ?? null,
    description: s.description ?? null,
  };
}

function mapChannel(c: Raw) {
  return {
    id: c.id,
    serverId: c.server_id ?? null,
    name: (c.name as string) ?? "",
    kind: c.channel_type,
    isE2ee: c.encrypted ?? false,
    disappearAfterSeconds: (c.disappear_after_seconds as number) ?? 0,
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

// ── Phase 13 mappers ──────────────────────────────────────────────────────────

function mapPoll(p: Raw) {
  return {
    id: p.id as string,
    channelId: p.channel_id as string,
    messageId: (p.message_id as string | undefined) ?? undefined,
    authorId: p.author_id as string,
    question: p.question as string,
    options: (p.options as string[]) ?? [],
    endsAt: (p.ends_at as string | undefined) ?? undefined,
    allowMultiselect: (p.allow_multiselect as boolean) ?? false,
    isAnonymous: (p.is_anonymous as boolean) ?? false,
    status: (p.status as "open" | "ended") ?? "open",
    createdAt: p.created_at as string,
    updatedAt: p.updated_at as string,
  };
}

function mapScheduledMessage(m: Raw) {
  return {
    id: m.id as string,
    channelId: m.channel_id as string,
    authorId: m.author_id as string,
    content: m.content as string,
    scheduledAt: m.scheduled_at as string,
    status: (m.status as "pending" | "sent" | "failed" | "cancelled") ?? "pending",
    createdAt: m.created_at as string,
  };
}

function mapBookmark(b: Raw) {
  const msg = b.message as Raw | undefined;
  return {
    id: b.id as string,
    userId: b.user_id as string,
    messageId: b.message_id as string,
    channelId: b.channel_id as string,
    note: (b.note as string | undefined) ?? undefined,
    createdAt: b.created_at as string,
    message: msg
      ? {
          id: msg.id as string,
          content: msg.content as string,
          authorId: msg.author_id as string,
          authorUsername: msg.author_username as string,
          channelId: msg.channel_id as string,
          createdAt: msg.created_at as string,
        }
      : undefined,
  };
}

// ── v0.14 mappers ────────────────────────────────────────────────────────────

function mapServerEvent(e: Raw) {
  return {
    id: e.id as string,
    serverId: e.server_id as string,
    creatorId: e.creator_id as string,
    title: e.title as string,
    description: (e.description as string | undefined) ?? undefined,
    startsAt: e.starts_at as string,
    endsAt: (e.ends_at as string | undefined) ?? undefined,
    location: (e.location as string | undefined) ?? undefined,
    channelId: (e.channel_id as string | undefined) ?? undefined,
    coverImage: (e.cover_image as string | undefined) ?? undefined,
    status: (e.status as "scheduled" | "active" | "completed" | "cancelled") ?? "scheduled",
    interestedCount: (e.interested_count as number) ?? 0,
    isInterested: (e.is_interested as boolean) ?? false,
    createdAt: e.created_at as string,
    updatedAt: e.updated_at as string,
  };
}

function mapSticker(s: Raw) {
  return {
    id: s.id as string,
    packId: s.pack_id as string,
    serverId: (s.server_id as string | undefined) ?? undefined,
    name: s.name as string,
    description: (s.description as string | undefined) ?? undefined,
    assetUrl: s.asset_url as string,
    type: (s.type as "png" | "apng" | "lottie") ?? "png",
    createdAt: s.created_at as string,
  };
}

function mapStickerPack(p: Raw) {
  return {
    id: p.id as string,
    name: p.name as string,
    description: (p.description as string | undefined) ?? undefined,
    coverStickerId: (p.cover_sticker_id as string | undefined) ?? undefined,
    serverId: (p.server_id as string | undefined) ?? undefined,
    isPremium: (p.is_premium as boolean) ?? false,
    stickers: Array.isArray(p.stickers) ? (p.stickers as Raw[]).map(mapSticker) : undefined,
  };
}

// ── v0.15 Community Ecosystem mappers ─────────────────────────────────

function mapUserBadge(b: Raw) {
  return {
    id: b.id as string,
    userId: b.user_id as string,
    badgeType: b.badge_type as string,
    serverId: (b.server_id as string | undefined) ?? undefined,
    awardedBy: (b.awarded_by as string | undefined) ?? undefined,
    awardedAt: b.awarded_at as string,
    label: (b.label as string | undefined) ?? undefined,
    iconUrl: (b.icon_url as string | undefined) ?? undefined,
  };
}

function mapBoosterEntry(b: Raw) {
  return {
    id: b.id as string,
    userId: b.user_id as string,
    serverId: b.server_id as string,
    slot: b.slot as number,
    startedAt: b.started_at as string,
    expiresAt: (b.expires_at as string | undefined) ?? undefined,
  };
}

function mapBoostTierInfo(t: Raw) {
  return {
    tier: t.tier as number,
    boosterCount: t.booster_count as number,
    extraEmojiSlots: t.extra_emoji_slots as number,
    uploadLimitBytes: t.upload_limit_bytes as number,
    vanityUrlAvailable: t.vanity_url_available as boolean,
    currentVanityCode: (t.current_vanity_code as string | undefined) ?? undefined,
  };
}

function mapCanvasBlock(b: Raw) {
  return {
    id: b.id as string,
    channelId: b.channel_id as string,
    blockType: b.block_type as string,
    content: b.content as Record<string, unknown>,
    position: b.position as number,
    updatedBy: (b.updated_by as string | undefined) ?? undefined,
    updatedAt: b.updated_at as string,
  };
}

// ── Federation mappers (v0.8.5) ───────────────────────────────────────────────

function mapFederationStatus(r: Raw) {
  return {
    serverName:             r.server_name as string,
    softwareVersion:        (r.software_version as string) ?? "",
    federationEnabled:      Boolean(r.federation_enabled),
    peerCount:              (r.peer_count as number) ?? 0,
    healthyPeerCount:       (r.healthy_peer_count as number) ?? 0,
    pendingInboundRequests: (r.pending_inbound_requests as number) ?? 0,
    pendingOutboundRequests:(r.pending_outbound_requests as number) ?? 0,
    uptimeSeconds:          (r.uptime_seconds as number) ?? 0,
  };
}

function mapFederationIdentity(r: Raw) {
  return {
    displayName:       (r.display_name as string | undefined) ?? undefined,
    description:       (r.description as string | undefined) ?? undefined,
    adminContact:      (r.admin_contact as string | undefined) ?? undefined,
    federationPolicy:  (r.federation_policy as string) ?? "open",
  };
}

function mapFederatedPeer(r: Raw) {
  return {
    domain:       r.domain as string,
    displayName:  (r.display_name as string | undefined) ?? undefined,
    trustScore:   (r.trust_score as number) ?? 50,
    isHealthy:    Boolean(r.is_healthy),
    isBlocked:    Boolean(r.is_blocked),
    latencyMs:    (r.latency_ms as number | undefined) ?? undefined,
    lastSeenAt:   (r.last_seen_at as string | undefined) ?? undefined,
    notes:        (r.notes as string | undefined) ?? undefined,
    addedAt:      (r.added_at as string) ?? "",
  };
}

function mapPeeringRequest(r: Raw) {
  return {
    id:                 r.id as string,
    direction:          r.direction as "inbound" | "outbound",
    remoteDomain:       r.remote_domain as string,
    remoteDisplayName:  (r.remote_display_name as string | undefined) ?? undefined,
    remoteDescription:  (r.remote_description as string | undefined) ?? undefined,
    status:             r.status as "pending" | "accepted" | "rejected" | "cancelled",
    message:            (r.message as string | undefined) ?? undefined,
    createdAt:          (r.created_at as string) ?? "",
  };
}

function mapAuditEntry(r: Raw) {
  return {
    id:           r.id as string,
    adminId:      (r.admin_id as string) ?? "",
    action:       r.action as string,
    targetDomain: (r.target_domain as string) ?? "",
    details:      (r.details as Record<string, unknown>) ?? {},
    createdAt:    (r.created_at as string) ?? "",
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

    case "refresh_token": {
      if (!_refreshToken) throw new Error("No refresh token");
      const resp = await apiFetch<Raw>("POST", "/api/v1/auth/refresh", {
        refresh_token: _refreshToken,
      });
      _token = resp.access_token as string;
      localStorage.setItem("nexus:dev:token", _token);
      if (typeof resp.refresh_token === "string") {
        _refreshToken = resp.refresh_token;
        localStorage.setItem("nexus:dev:refreshToken", _refreshToken);
      }
      return _token as unknown as T;
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
        tags: args.tags ?? undefined,
        category: args.category ?? undefined,
      });
      return mapServer(raw) as T;
    }

    case "update_server": {
      const raw = await apiFetch<Raw>("PATCH", `/api/v1/servers/${args.serverId}`, {
        name: args.name ?? undefined,
        description: args.description ?? undefined,
        is_public: args.isPublic ?? undefined,
        region: args.region ?? undefined,
        require_2fa: args.require2fa ?? undefined,
        spam_window_secs: args.spamWindowSecs ?? undefined,
        spam_max_messages: args.spamMaxMessages ?? undefined,
        tags: args.tags ?? undefined,
        category: args.category ?? undefined,
        tip_jar_url: args.tipJarUrl ?? undefined,
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

    // ── Moderation ────────────────────────────────────────────────────────
    case "get_audit_log": {
      const params = new URLSearchParams();
      if (args.action) params.set("action", String(args.action));
      if (args.actorId) params.set("actor", String(args.actorId));
      if (args.limit) params.set("limit", String(args.limit));
      const qs = params.toString();
      return apiFetch<T>(
        "GET",
        `/api/v1/servers/${args.serverId}/audit-log${qs ? "?" + qs : ""}`
      );
    }

    // ── Session management ────────────────────────────────────────────────
    case "list_sessions": {
      return apiFetch<T>("GET", `/api/v1/auth/sessions`);
    }

    case "revoke_session": {
      return apiFetch<T>("DELETE", `/api/v1/auth/sessions/${args.sessionId}`);
    }

    case "revoke_all_sessions": {
      return apiFetch<T>("DELETE", `/api/v1/auth/sessions`);
    }

    // ── Server ownership transfer ─────────────────────────────────────────
    case "transfer_server_ownership": {
      return apiFetch<T>("POST", `/api/v1/servers/${args.serverId}/transfer-ownership`, {
        new_owner_id: args.newOwnerId,
      });
    }

    // ── Desktop-only commands (no-ops in browser) ─────────────────────────
    case "install_update":
      console.info("[browser] install_update is a no-op in the browser");
      return undefined as unknown as T;

    // ── Phase 13: Polls ───────────────────────────────────────────────────
    case "list_channel_polls": {
      const raw = await apiFetch<Raw[]>("GET", `/api/v1/channels/${args.channelId}/polls`);
      return raw.map(mapPoll) as unknown as T;
    }

    case "cast_poll_vote": {
      return apiFetch<T>("POST", `/api/v1/channels/${args.channelId}/polls/${args.pollId}/vote`, {
        option_indices: args.optionIndices,
      });
    }

    case "retract_poll_vote": {
      return apiFetch<T>("DELETE", `/api/v1/channels/${args.channelId}/polls/${args.pollId}/vote`);
    }

    case "get_poll_results": {
      return apiFetch<T>("GET", `/api/v1/channels/${args.channelId}/polls/${args.pollId}/results`);
    }

    case "create_poll": {
      const raw = await apiFetch<Raw>("POST", `/api/v1/channels/${args.channelId}/polls`, {
        question: args.question,
        options: args.options,
        ends_at: args.endsAt ?? null,
        allow_multiselect: args.allowMultiselect ?? false,
        is_anonymous: args.isAnonymous ?? false,
      });
      return mapPoll(raw) as unknown as T;
    }

    // ── Phase 13: Bookmarks ───────────────────────────────────────────────
    case "list_bookmarks": {
      const raw = await apiFetch<Raw[]>("GET", "/api/v1/users/@me/bookmarks");
      return raw.map(mapBookmark) as unknown as T;
    }

    case "add_bookmark": {
      const raw = await apiFetch<Raw>("POST", "/api/v1/users/@me/bookmarks", {
        message_id: args.messageId,
        channel_id: args.channelId,
        note: args.note ?? null,
      });
      return mapBookmark(raw) as unknown as T;
    }

    case "remove_bookmark": {
      return apiFetch<T>("DELETE", `/api/v1/users/@me/bookmarks/${args.messageId}`);
    }

    // ── Phase 13: Drafts ──────────────────────────────────────────────────
    case "get_draft": {
      return apiFetch<T>("GET", `/api/v1/channels/${args.channelId}/draft`);
    }

    case "save_draft": {
      return apiFetch<T>("PUT", `/api/v1/channels/${args.channelId}/draft`, {
        content: args.content,
      });
    }

    case "delete_draft": {
      return apiFetch<T>("DELETE", `/api/v1/channels/${args.channelId}/draft`);
    }

    // ── Phase 13: Scheduled Messages ──────────────────────────────────────
    case "list_scheduled_messages": {
      const raw = await apiFetch<Raw[]>("GET", `/api/v1/channels/${args.channelId}/scheduled-messages`);
      return raw.map(mapScheduledMessage) as unknown as T;
    }

    case "create_scheduled_message": {
      const raw = await apiFetch<Raw>("POST", `/api/v1/channels/${args.channelId}/scheduled-messages`, {
        content: args.content,
        scheduled_at: args.scheduledAt,
      });
      return mapScheduledMessage(raw) as unknown as T;
    }

    case "update_scheduled_message": {
      const raw = await apiFetch<Raw>("PATCH", `/api/v1/channels/${args.channelId}/scheduled-messages/${args.scheduledMessageId}`, {
        content: args.content ?? undefined,
        scheduled_at: args.scheduledAt ?? undefined,
      });
      return mapScheduledMessage(raw) as unknown as T;
    }

    case "cancel_scheduled_message": {
      return apiFetch<T>("DELETE", `/api/v1/channels/${args.channelId}/scheduled-messages/${args.scheduledMessageId}`);
    }

    // ── Phase 13: Note-to-self ────────────────────────────────────────────
    case "get_note_to_self_channel": {
      const raw = await apiFetch<Raw>("GET", "/api/v1/users/@me/note-to-self");
      return { channelId: raw.channel_id as string } as unknown as T;
    }

    // ── Phase 13: Presence / Status ───────────────────────────────────────
    case "update_presence_status": {
      return apiFetch<T>("POST", "/api/v1/users/@me/presence", {
        presence: args.presence ?? null,
        status: args.customStatus ?? null,
        custom_status_emoji: args.customStatusEmoji ?? null,
        custom_status_expires_at: args.expiresAt ?? null,
      });
    }

    // ── Phase 14: Message Forwarding ─────────────────────────────────────
    case "forward_message": {
      return apiFetch<T>("POST", `/api/v1/messages/${args.messageId}/forward`, {
        target_channel_ids: args.targetChannelIds,
      });
    }

    // ── Phase 14: Server Events ───────────────────────────────────────────
    case "list_server_events": {
      const qs = args.status ? `?status=${args.status}` : "";
      const rows = await apiFetch<Raw[]>("GET", `/api/v1/servers/${args.serverId}/events${qs}`);
      return rows.map(mapServerEvent) as unknown as T;
    }

    case "get_server_event": {
      const raw = await apiFetch<Raw>("GET", `/api/v1/servers/${args.serverId}/events/${args.eventId}`);
      return mapServerEvent(raw) as unknown as T;
    }

    case "create_server_event": {
      const raw = await apiFetch<Raw>("POST", `/api/v1/servers/${args.serverId}/events`, {
        title: args.title,
        description: args.description ?? undefined,
        starts_at: args.startsAt,
        ends_at: args.endsAt ?? undefined,
        location: args.location ?? undefined,
        channel_id: args.channelId ?? undefined,
      });
      return mapServerEvent(raw) as unknown as T;
    }

    case "update_server_event": {
      const raw = await apiFetch<Raw>("PATCH", `/api/v1/servers/${args.serverId}/events/${args.eventId}`, {
        title: args.title ?? undefined,
        description: args.description ?? undefined,
        starts_at: args.startsAt ?? undefined,
        ends_at: args.endsAt ?? undefined,
        location: args.location ?? undefined,
      });
      return mapServerEvent(raw) as unknown as T;
    }

    case "cancel_server_event": {
      return apiFetch<T>("DELETE", `/api/v1/servers/${args.serverId}/events/${args.eventId}`);
    }

    case "rsvp_server_event": {
      return apiFetch<T>("PUT", `/api/v1/servers/${args.serverId}/events/${args.eventId}/interested`);
    }

    case "un_rsvp_server_event": {
      return apiFetch<T>("DELETE", `/api/v1/servers/${args.serverId}/events/${args.eventId}/interested`);
    }

    // ── Phase 14: Sticker Packs ───────────────────────────────────────────
    case "list_sticker_packs": {
      const rows = await apiFetch<Raw[]>("GET", "/api/v1/sticker-packs");
      return rows.map(mapStickerPack) as unknown as T;
    }

    case "list_server_stickers": {
      const rows = await apiFetch<Raw[]>("GET", `/api/v1/servers/${args.serverId}/stickers`);
      return rows.map(mapSticker) as unknown as T;
    }

    case "delete_sticker": {
      return apiFetch<T>("DELETE", `/api/v1/servers/${args.serverId}/stickers/${args.stickerId}`);
    }

    // ── Phase 14: Inline Bot Queries ──────────────────────────────────────
    case "inline_query": {
      const qs = args.botId
        ? `?query=${encodeURIComponent(args.query as string)}&bot_id=${args.botId}`
        : `?query=${encodeURIComponent(args.query as string)}`;
      return apiFetch<T>("GET", `/api/v1/channels/${args.channelId}/inline-query${qs}`);
    }

    case "list_inline_triggers": {
      return apiFetch<T>("GET", `/api/v1/channels/${args.channelId}/bots/inline-triggers`);
    }

    // ── Phase 15: User Badges ───────────────────────────────────────────────
    case "list_user_badges": {
      const rows = await apiFetch<Raw[]>("GET", `/api/v1/users/${args.userId}/badges`);
      return rows.map(mapUserBadge) as unknown as T;
    }

    case "award_badge": {
      const raw = await apiFetch<Raw>("POST", `/api/v1/admin/users/${args.userId}/badges`, {
        badge_type: args.badgeType,
        server_id: args.serverId ?? undefined,
        label: args.label ?? undefined,
        icon_url: args.iconUrl ?? undefined,
      });
      return mapUserBadge(raw) as unknown as T;
    }

    case "revoke_badge": {
      return apiFetch<T>("DELETE", `/api/v1/admin/users/${args.userId}/badges`, {
        badge_type: args.badgeType,
      });
    }

    // ── Phase 15: Server Boosters ──────────────────────────────────────────
    case "boost_server": {
      const raw = await apiFetch<Raw>("POST", `/api/v1/servers/${args.serverId}/boost`);
      return mapBoosterEntry(raw) as unknown as T;
    }

    case "remove_boost": {
      return apiFetch<T>("DELETE", `/api/v1/servers/${args.serverId}/boost/${args.slot}`);
    }

    case "list_server_boosters": {
      const rows = await apiFetch<Raw[]>("GET", `/api/v1/servers/${args.serverId}/boosters`);
      return rows.map(mapBoosterEntry) as unknown as T;
    }

    case "get_server_boost_tier": {
      const raw = await apiFetch<Raw>("GET", `/api/v1/servers/${args.serverId}/boost-tier`);
      return mapBoostTierInfo(raw) as unknown as T;
    }

    case "set_vanity_url": {
      return apiFetch<T>("PATCH", `/api/v1/servers/${args.serverId}/vanity-url`, {
        code: args.code,
      });
    }

    // ── Phase 15: Canvas ────────────────────────────────────────────────────────────
    case "get_canvas": {
      const rows = await apiFetch<Raw[]>("GET", `/api/v1/channels/${args.channelId}/canvas`);
      return rows.map(mapCanvasBlock) as unknown as T;
    }

    case "upsert_canvas_block": {
      const raw = await apiFetch<Raw>("PUT",
        `/api/v1/channels/${args.channelId}/canvas/blocks/${args.blockId}`, {
          block_type: args.blockType ?? "paragraph",
          content: args.content,
          position: args.position ?? undefined,
        }
      );
      return mapCanvasBlock(raw) as unknown as T;
    }

    case "delete_canvas_block": {
      return apiFetch<T>("DELETE", `/api/v1/channels/${args.channelId}/canvas/blocks/${args.blockId}`);
    }

    case "reorder_canvas_blocks": {
      return apiFetch<T>("POST", `/api/v1/channels/${args.channelId}/canvas/blocks/reorder`, {
        blocks: args.blocks,
      });
    }

    // ── Federation (v0.8.5) ───────────────────────────────────────────────

    case "get_federation_status": {
      const raw = await apiFetch<Raw>("GET", "/api/v1/admin/federation/status");
      return mapFederationStatus(raw) as unknown as T;
    }

    case "get_federation_identity": {
      const raw = await apiFetch<Raw>("GET", "/api/v1/admin/federation/identity");
      return mapFederationIdentity(raw) as unknown as T;
    }

    case "update_federation_identity": {
      const raw = await apiFetch<Raw>("PATCH", "/api/v1/admin/federation/identity", {
        display_name:       args.displayName,
        description:        args.description,
        admin_contact:      args.adminContact,
        federation_policy:  args.federationPolicy,
      });
      return mapFederationIdentity(raw) as unknown as T;
    }

    case "list_peers": {
      const raw = await apiFetch<{ peers: Raw[] }>("GET", "/api/v1/admin/federation/peers");
      return {
        peers: (raw.peers ?? []).map(mapFederatedPeer),
      } as unknown as T;
    }

    case "add_peer": {
      return apiFetch<T>("POST", "/api/v1/admin/federation/peers", {
        domain:      args.domain,
        message:     args.message,
        trust_score: args.trustScore,
      });
    }

    case "peer_health": {
      return apiFetch<T>("GET", `/api/v1/admin/federation/peers/${encodeURIComponent(args.domain as string)}/health`);
    }

    case "update_peer_trust": {
      return apiFetch<T>("PATCH", `/api/v1/admin/federation/peers/${encodeURIComponent(args.domain as string)}/trust`, {
        trust_score: args.trustScore,
        notes:       args.notes,
      });
    }

    case "block_peer": {
      return apiFetch<T>("POST", `/api/v1/admin/federation/peers/${encodeURIComponent(args.domain as string)}/block`);
    }

    case "unblock_peer": {
      return apiFetch<T>("POST", `/api/v1/admin/federation/peers/${encodeURIComponent(args.domain as string)}/unblock`);
    }

    case "remove_peer": {
      return apiFetch<T>("DELETE", `/api/v1/admin/federation/peers/${encodeURIComponent(args.domain as string)}`);
    }

    case "list_peering_requests": {
      const params = new URLSearchParams();
      if (args.status) params.set("status", args.status as string);
      if (args.direction) params.set("direction", args.direction as string);
      const qs = params.toString() ? `?${params}` : "";
      const raw = await apiFetch<{ requests: Raw[]; total: number }>(
        "GET", `/api/v1/admin/federation/requests${qs}`
      );
      return {
        requests: (raw.requests ?? []).map(mapPeeringRequest),
        total: raw.total ?? 0,
      } as unknown as T;
    }

    case "accept_peering_request": {
      return apiFetch<T>("POST", `/api/v1/admin/federation/requests/${args.requestId}/accept`);
    }

    case "reject_peering_request": {
      return apiFetch<T>("POST", `/api/v1/admin/federation/requests/${args.requestId}/reject`);
    }

    case "get_federation_audit": {
      const params = new URLSearchParams();
      if (args.domain) params.set("domain", args.domain as string);
      if (args.limit) params.set("limit", String(args.limit));
      const qs = params.toString() ? `?${params}` : "";
      const raw = await apiFetch<{ entries: Raw[]; total: number }>(
        "GET", `/api/v1/admin/federation/audit${qs}`
      );
      return {
        entries: (raw.entries ?? []).map(mapAuditEntry),
        total: raw.total ?? 0,
      } as unknown as T;
    }

    case "search_federated_users": {
      const params = new URLSearchParams();
      if (args.query) params.set("q", args.query as string);
      if (args.domain) params.set("domain", args.domain as string);
      const qs = params.toString() ? `?${params}` : "";
      return apiFetch<T>("GET", `/api/v1/federation/search${qs}`);
    }

    // ── Server Directory / Federated room browser ─────────────────────────
    case "directory_list_servers": {
      const limit = args.limit ?? 50;
      return apiFetch<T>("GET", `/api/v1/directory/servers?limit=${limit}`);
    }

    case "directory_list_rooms": {
      const params = new URLSearchParams();
      params.set("limit", String(args.limit ?? 50));
      if (args.server) params.set("server", args.server as string);
      return apiFetch<T>("GET", `/api/v1/directory/rooms?${params}`);
    }

    case "directory_search_rooms": {
      const params = new URLSearchParams();
      params.set("q", String(args.query ?? ""));
      params.set("limit", String(args.limit ?? 50));
      if (args.server) params.set("server", args.server as string);
      return apiFetch<T>("GET", `/api/v1/directory/rooms/search?${params}`);
    }

    case "directory_join_room": {
      return apiFetch<T>("POST", "/api/v1/directory/rooms/join", { room_id: args.roomId });
    }

    // ── Discovery (v0.16) ─────────────────────────────────────────────────
    case "discover_browse_servers": {
      const params = new URLSearchParams();
      params.set("limit", String(args.limit ?? 20));
      params.set("offset", String(args.offset ?? 0));
      if (args.tag) params.set("tag", args.tag as string);
      if (args.category) params.set("category", args.category as string);
      const resp = await apiFetch<Raw>("GET", `/api/v1/discover/servers?${params}`);
      return { ...resp, servers: ((resp as Record<string, unknown>).servers as Raw[]).map(mapServer) } as T;
    }

    case "discover_featured_servers": {
      const resp = await apiFetch<Raw>("GET", "/api/v1/discover/servers/featured");
      return { ...resp, servers: ((resp as Record<string, unknown>).servers as Raw[]).map(mapServer) } as T;
    }

    case "discover_search_servers": {
      const params = new URLSearchParams();
      params.set("q", String(args.query ?? ""));
      params.set("limit", String(args.limit ?? 20));
      params.set("offset", String(args.offset ?? 0));
      const resp = await apiFetch<Raw>("GET", `/api/v1/discover/servers/search?${params}`);
      return { ...resp, servers: ((resp as Record<string, unknown>).servers as Raw[]).map(mapServer) } as T;
    }

    case "discover_server_preview": {
      const resp = await apiFetch<Raw>("GET", `/api/v1/discover/servers/${args.serverId}/preview`);
      // Flatten: the server fields are at the root level due to #[serde(flatten)]
      return resp as T;
    }

    case "discover_categories": {
      return apiFetch<T>("GET", "/api/v1/discover/categories");
    }

    case "discover_feature_server": {
      const raw = await apiFetch<Raw>(
        "POST",
        `/api/v1/discover/servers/${args.serverId}/feature`,
        { featured: args.featured ?? true },
      );
      return mapServer(raw) as T;
    }

    // ── Monetization (v0.16) ──────────────────────────────────────────────
    case "get_monetization_config": {
      return apiFetch<T>("GET", `/api/v1/servers/${args.serverId}/monetization`);
    }

    case "update_monetization_config": {
      return apiFetch<T>("PUT", `/api/v1/servers/${args.serverId}/monetization`, {
        provider: args.provider ?? undefined,
        webhook_url: args.webhookUrl ?? undefined,
        enabled: args.enabled ?? undefined,
        tip_jar_url: args.tipJarUrl ?? undefined,
      });
    }

    case "list_subscription_tiers": {
      return apiFetch<T>("GET", `/api/v1/servers/${args.serverId}/subscription-tiers`);
    }

    case "create_subscription_tier": {
      return apiFetch<T>("POST", `/api/v1/servers/${args.serverId}/subscription-tiers`, {
        name: args.name,
        description: args.description ?? undefined,
        price_cents: args.priceCents,
        currency: args.currency ?? undefined,
        interval: args.interval ?? undefined,
        role_id: args.roleId ?? undefined,
        position: args.position ?? undefined,
      });
    }

    case "get_creator_analytics": {
      const days = args.days ?? 30;
      return apiFetch<T>("GET", `/api/v1/servers/${args.serverId}/analytics?days=${days}`);
    }

    // ── Tasks (v1.5 — 16-01) ─────────────────────────────────────────────
    case "list_tasks": {
      let url = `/api/v1/servers/${args.serverId}/tasks?channel_id=${args.channelId}`;
      if (args.status) url += `&status=${args.status}`;
      return apiFetch<T>("GET", url);
    }
    case "create_task": {
      return apiFetch<T>("POST", `/api/v1/servers/${args.serverId}/tasks`, {
        channel_id: args.channelId,
        title: args.title,
        description: args.description ?? undefined,
        priority: args.priority ?? undefined,
        assignee_id: args.assigneeId ?? undefined,
        due_at: args.dueAt ?? undefined,
      });
    }
    case "get_task": {
      return apiFetch<T>("GET", `/api/v1/servers/${args.serverId}/tasks/${args.taskId}`);
    }
    case "update_task": {
      return apiFetch<T>("PUT", `/api/v1/servers/${args.serverId}/tasks/${args.taskId}`, args.body);
    }
    case "delete_task": {
      return apiFetch<T>("DELETE", `/api/v1/servers/${args.serverId}/tasks/${args.taskId}`);
    }
    case "my_tasks": {
      return apiFetch<T>("GET", "/api/v1/users/@me/tasks");
    }
    case "add_checklist_item": {
      return apiFetch<T>("POST", `/api/v1/tasks/${args.taskId}/checklist`, {
        content: args.content,
      });
    }
    case "list_checklist": {
      return apiFetch<T>("GET", `/api/v1/tasks/${args.taskId}/checklist`);
    }
    case "toggle_checklist_item": {
      return apiFetch<T>("PUT", `/api/v1/tasks/${args.taskId}/checklist/${args.itemId}`);
    }
    case "delete_checklist_item": {
      return apiFetch<T>("DELETE", `/api/v1/tasks/${args.taskId}/checklist/${args.itemId}`);
    }
    case "create_task_reminder": {
      return apiFetch<T>("POST", `/api/v1/tasks/${args.taskId}/reminders`, {
        remind_at: args.remindAt,
      });
    }
    case "my_task_reminders": {
      return apiFetch<T>("GET", "/api/v1/users/@me/reminders");
    }
    case "delete_task_reminder": {
      return apiFetch<T>("DELETE", `/api/v1/reminders/${args.reminderId}`);
    }

    // ── Calendar (v1.5 — 16-02) ──────────────────────────────────────────
    case "list_calendar_events": {
      let url = `/api/v1/servers/${args.serverId}/calendar`;
      const params: string[] = [];
      if (args.from) params.push(`from=${args.from}`);
      if (args.to) params.push(`to=${args.to}`);
      if (params.length) url += `?${params.join("&")}`;
      return apiFetch<T>("GET", url);
    }
    case "create_calendar_event": {
      return apiFetch<T>("POST", `/api/v1/servers/${args.serverId}/calendar`, {
        title: args.title,
        description: args.description ?? undefined,
        location: args.location ?? undefined,
        starts_at: args.startsAt,
        ends_at: args.endsAt,
        all_day: args.allDay ?? false,
        rrule: args.rrule ?? undefined,
        color: args.color ?? undefined,
        channel_id: args.channelId ?? undefined,
      });
    }
    case "get_calendar_event": {
      return apiFetch<T>("GET", `/api/v1/servers/${args.serverId}/calendar/${args.eventId}`);
    }
    case "update_calendar_event": {
      return apiFetch<T>("PUT", `/api/v1/servers/${args.serverId}/calendar/${args.eventId}`, args.body);
    }
    case "delete_calendar_event": {
      return apiFetch<T>("DELETE", `/api/v1/servers/${args.serverId}/calendar/${args.eventId}`);
    }
    case "calendar_rsvp": {
      return apiFetch<T>("POST", `/api/v1/calendar/${args.eventId}/rsvp`, {
        status: args.status,
      });
    }
    case "list_calendar_rsvps": {
      return apiFetch<T>("GET", `/api/v1/calendar/${args.eventId}/rsvp`);
    }
    case "remove_calendar_rsvp": {
      return apiFetch<T>("DELETE", `/api/v1/calendar/${args.eventId}/rsvp`);
    }
    case "export_calendar_ics": {
      return apiFetch<T>("GET", `/api/v1/servers/${args.serverId}/calendar.ics`);
    }

    // ── File Versions (v1.5 — 16-03) ─────────────────────────────────────
    case "list_file_versions": {
      return apiFetch<T>("GET", `/api/v1/attachments/${args.attachmentId}/versions`);
    }
    case "create_file_version": {
      return apiFetch<T>("POST", `/api/v1/attachments/${args.attachmentId}/versions`, {
        filename: args.filename,
        content_type: args.contentType ?? undefined,
        size: args.size,
        storage_key: args.storageKey,
        sha256: args.sha256 ?? undefined,
        comment: args.comment ?? undefined,
      });
    }
    case "get_file_version": {
      return apiFetch<T>("GET", `/api/v1/attachments/${args.attachmentId}/versions/${args.version}`);
    }
    case "delete_file_version": {
      return apiFetch<T>("DELETE", `/api/v1/attachments/${args.attachmentId}/versions/${args.version}`);
    }
    case "get_server_storage_quota": {
      return apiFetch<T>("GET", `/api/v1/servers/${args.serverId}/storage`);
    }
    case "set_server_storage_quota": {
      return apiFetch<T>("PUT", `/api/v1/servers/${args.serverId}/storage`, {
        max_bytes: args.maxBytes,
      });
    }

    // ── AI Assists (v1.5 — 16-04) ────────────────────────────────────────
    case "get_ai_preferences": {
      return apiFetch<T>("GET", "/api/v1/users/@me/ai-preferences");
    }
    case "update_ai_preferences": {
      return apiFetch<T>("PUT", "/api/v1/users/@me/ai-preferences", args.body);
    }
    case "list_channel_digests": {
      let url = `/api/v1/channels/${args.channelId}/digests`;
      if (args.limit) url += `?limit=${args.limit}`;
      return apiFetch<T>("GET", url);
    }
    case "save_channel_digest": {
      return apiFetch<T>("POST", `/api/v1/channels/${args.channelId}/digests`, {
        period_start: args.periodStart,
        period_end: args.periodEnd,
        summary: args.summary,
        message_count: args.messageCount ?? undefined,
      });
    }

    default:
      throw new Error(`[browser] Unhandled invoke command: "${cmd}"`);
  }
}

// ── Public API ────────────────────────────────────────────────────────────────

export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  // Keep JS-side _serverUrl in sync regardless of runtime mode so that
  // getServerUrl() always returns the correct value for invite links etc.
  // Also persist to localStorage so Login/Register pages can pre-fill it.
  if (cmd === "set_server_url" && args?.url) {
    _serverUrl = args.url as string;
    localStorage.setItem("nexus:lastServerUrl", _serverUrl);
  }

  if (isTauri()) {
    // Dynamic import so the Tauri module is never bundled when running in browser
    const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
    return tauriInvoke<T>(cmd, args);
  }
  return browserInvoke<T>(cmd, args ?? {});
}

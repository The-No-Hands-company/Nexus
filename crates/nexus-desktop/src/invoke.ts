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

function authHeaders(): Record<string, string> {
  const h: Record<string, string> = { "Content-Type": "application/json" };
  if (_token) h["Authorization"] = `Bearer ${_token}`;
  return h;
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
      return resp as T;
    }

    case "logout": {
      _token = null;
      localStorage.removeItem("nexus:dev:token");
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

    case "update_profile": {
      return apiFetch<T>("PATCH", "/api/v1/users/me", {
        display_name: args.displayName ?? undefined,
        avatar_url: args.avatarUrl ?? undefined,
      });
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

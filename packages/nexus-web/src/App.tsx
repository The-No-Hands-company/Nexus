import { FormEvent, useEffect, useMemo, useRef, useState } from "react";

type Session = {
  accessToken: string;
  username: string;
};

type Server = { id: string; name: string };

type Channel = {
  id: string;
  name: string;
  channel_type?: string;
};

type Message = {
  id: string;
  content: string;
  author_username?: string;
  created_at?: string;
};

type ExperienceMode = "full" | "messaging";

type ExperienceProfile = {
  experience_mode?: ExperienceMode;
};

type GatewayEnvelope = {
  op: string;
  d?: unknown;
};

const API_BASE = localStorage.getItem("nexus_api_base") || "http://localhost:8080/api/v1";

function authHeaders(session: Session | null): HeadersInit {
  if (!session) {
    return { "Content-Type": "application/json" };
  }
  return {
    "Content-Type": "application/json",
    Authorization: `Bearer ${session.accessToken}`,
  };
}

export default function App() {
  const [apiBase, setApiBase] = useState(API_BASE);
  const [session, setSession] = useState<Session | null>(null);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [mode, setMode] = useState<"login" | "register">("login");
  const [error, setError] = useState<string | null>(null);

  const [servers, setServers] = useState<Server[]>([]);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [activeServer, setActiveServer] = useState<string | null>(null);
  const [activeChannel, setActiveChannel] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [gatewayStatus, setGatewayStatus] = useState<"offline" | "connecting" | "online">("offline");
  const [experienceMode, setExperienceMode] = useState<ExperienceMode>("full");
  const [savingMode, setSavingMode] = useState(false);
  const activeChannelRef = useRef<string | null>(null);

  const authPath = mode === "login" ? "/auth/login" : "/auth/register";

  const statusText = useMemo(() => {
    if (!session) return "Not authenticated";
    return `Signed in as ${session.username}`;
  }, [session]);

  useEffect(() => {
    localStorage.setItem("nexus_api_base", apiBase);
  }, [apiBase]);

  useEffect(() => {
    activeChannelRef.current = activeChannel;
  }, [activeChannel]);

  useEffect(() => {
    if (!session) return;
    void loadServers(session, apiBase, setServers, setError);
  }, [session, apiBase]);

  useEffect(() => {
    if (!session) return;
    void loadExperienceProfile(session, apiBase, setExperienceMode, setError);
  }, [session, apiBase]);

  useEffect(() => {
    if (!activeServer && servers.length > 0) {
      setActiveServer(servers[0].id);
    }
  }, [activeServer, servers]);

  useEffect(() => {
    if (!session || !activeServer) return;
    void loadChannels(session, apiBase, activeServer, setChannels, setError);
  }, [session, apiBase, activeServer]);

  useEffect(() => {
    if (!session || !activeChannel) return;
    void loadMessages(session, apiBase, activeChannel, setMessages, setError);
  }, [session, apiBase, activeChannel]);

  useEffect(() => {
    if (!session) {
      setGatewayStatus("offline");
      return;
    }

    const gatewayBase = toGatewayBase(apiBase);
    if (!gatewayBase) {
      setError("Unable to derive gateway URL from API base URL");
      setGatewayStatus("offline");
      return;
    }

    let stopped = false;
    let ws: WebSocket | null = null;
    let heartbeatTimer: number | null = null;
    let reconnectTimer: number | null = null;
    let reconnectAttempt = 0;

    const clearTimers = () => {
      if (heartbeatTimer) {
        window.clearInterval(heartbeatTimer);
        heartbeatTimer = null;
      }
      if (reconnectTimer) {
        window.clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };

    const scheduleReconnect = () => {
      if (stopped) return;
      reconnectAttempt += 1;
      const backoffMs = Math.min(30_000, 1000 * 2 ** Math.min(reconnectAttempt, 5));
      setGatewayStatus("connecting");
      reconnectTimer = window.setTimeout(() => {
        connect();
      }, backoffMs);
    };

    const connect = () => {
      if (stopped) return;
      clearTimers();
      setGatewayStatus("connecting");

      ws = new WebSocket(`${gatewayBase}/gateway`);

      ws.onopen = () => {
        reconnectAttempt = 0;
        setGatewayStatus("online");
      };

      ws.onclose = () => {
        setGatewayStatus("offline");
        clearTimers();
        scheduleReconnect();
      };

      ws.onerror = () => {
        setGatewayStatus("offline");
      };

      ws.onmessage = (evt) => {
        try {
          const payload = JSON.parse(String(evt.data)) as GatewayEnvelope;

          if (payload.op === "Hello") {
            const heartbeatMs = Number((payload.d as { heartbeat_interval?: number } | undefined)?.heartbeat_interval ?? 45000);
            ws?.send(JSON.stringify({
              op: "Identify",
              d: { token: session.accessToken },
            }));

            if (heartbeatTimer) window.clearInterval(heartbeatTimer);
            heartbeatTimer = window.setInterval(() => {
              if (ws?.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({ op: "Heartbeat", d: { timestamp: Date.now() } }));
              }
            }, Math.max(heartbeatMs, 5000));
            return;
          }

          if (payload.op === "Dispatch") {
            const dispatch = payload.d as { event?: string; data?: Message & { channel_id?: string } } | undefined;
            if (!dispatch?.event || !dispatch.data) return;

            if (dispatch.event === "MESSAGE_CREATE") {
              const incoming = dispatch.data;
              if (!incoming.channel_id || incoming.channel_id !== activeChannelRef.current) return;

              setMessages((prev) => {
                if (prev.some((m) => m.id === incoming.id)) return prev;
                return [
                  {
                    id: incoming.id,
                    content: incoming.content,
                    author_username: incoming.author_username,
                    created_at: incoming.created_at,
                  },
                  ...prev,
                ];
              });
            }
          }
        } catch {
          // Ignore malformed gateway payloads.
        }
      };
    };

    connect();

    return () => {
      stopped = true;
      clearTimers();
      ws?.close();
    };
  }, [session, apiBase]);

  async function onAuthSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);

    try {
      const res = await fetch(`${apiBase}${authPath}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      if (!res.ok) {
        const text = await res.text();
        throw new Error(text || `Auth failed (${res.status})`);
      }

      const body = await res.json();
      const token = body.access_token;
      if (!token) {
        throw new Error("Token missing in auth response");
      }
      setSession({ accessToken: token, username });
    } catch (err) {
      setError((err as Error).message);
    }
  }

  async function onSendMessage() {
    if (!session || !activeChannel || !draft.trim()) return;

    try {
      const res = await fetch(`${apiBase}/channels/${activeChannel}/messages`, {
        method: "POST",
        headers: authHeaders(session),
        body: JSON.stringify({ content: draft }),
      });
      if (!res.ok) throw new Error(`Send failed (${res.status})`);
      setDraft("");
      await loadMessages(session, apiBase, activeChannel, setMessages, setError);
    } catch (err) {
      setError((err as Error).message);
    }
  }

  async function setMode(nextMode: ExperienceMode) {
    if (!session || nextMode === experienceMode || savingMode) return;
    const prev = experienceMode;
    setExperienceMode(nextMode);
    setSavingMode(true);
    try {
      await updateExperienceMode(session, apiBase, nextMode);
    } catch (err) {
      setExperienceMode(prev);
      setError((err as Error).message);
    } finally {
      setSavingMode(false);
    }
  }

  if (!session) {
    return (
      <div className="app">
        <form className="auth panel" onSubmit={onAuthSubmit}>
          <h2>Nexus Web</h2>
          <div className="meta">Browser client bootstrap for parity workstream</div>
          <label>
            API Base URL
            <input value={apiBase} onChange={(e) => setApiBase(e.target.value)} />
          </label>
          <label>
            Username
            <input value={username} onChange={(e) => setUsername(e.target.value)} required />
          </label>
          <label>
            Password
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </label>
          <div className="row">
            <button type="submit" className="grow">{mode === "login" ? "Sign In" : "Create Account"}</button>
            <button
              type="button"
              onClick={() => setMode((m) => (m === "login" ? "register" : "login"))}
            >
              {mode === "login" ? "Switch to Register" : "Switch to Login"}
            </button>
          </div>
          {error ? <div className="error">{error}</div> : null}
        </form>
      </div>
    );
  }

  return (
    <div className="app">
      <div className="meta" style={{ marginBottom: 8 }}>
        {statusText} · Gateway: {gatewayStatus} · Mode: {experienceMode}
      </div>
      <div className="mode-switch" style={{ marginBottom: 8 }}>
        <button type="button" disabled={savingMode || experienceMode === "full"} onClick={() => setMode("full")}>Full Experience</button>
        <button type="button" disabled={savingMode || experienceMode === "messaging"} onClick={() => setMode("messaging")}>Messaging Mode</button>
      </div>
      {error ? <div className="error" style={{ marginBottom: 8 }}>{error}</div> : null}
      <div className={`workspace ${experienceMode === "messaging" ? "messaging" : ""}`}>
        {experienceMode === "full" ? (
          <section className="panel col">
            <h3>Servers</h3>
            {servers.map((s) => (
              <button
                key={s.id}
                className="list-item"
                onClick={() => {
                  setActiveServer(s.id);
                  setActiveChannel(null);
                  setChannels([]);
                  setMessages([]);
                }}
              >
                {s.name}
              </button>
            ))}
          </section>
        ) : null}

        <section className="panel col">
          <h3>{experienceMode === "messaging" ? "Conversations" : "Channels"}</h3>
          {channels.map((c) => (
            <button
              key={c.id}
              className="list-item"
              onClick={() => setActiveChannel(c.id)}
            >
              {experienceMode === "messaging" ? c.name : `# ${c.name}`}
            </button>
          ))}
        </section>

        <section className="panel col">
          <h3>Messages</h3>
          <div style={{ marginBottom: 12 }}>
            {messages.map((m) => (
              <div className="message" key={m.id}>
                <div className="meta">{m.author_username || "unknown"}</div>
                <div>{m.content}</div>
              </div>
            ))}
          </div>
          <div className="row">
            <input
              className="grow"
              placeholder="Type a message"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              disabled={!activeChannel}
            />
            <button type="button" onClick={onSendMessage} disabled={!activeChannel}>Send</button>
          </div>
        </section>
      </div>
    </div>
  );
}

function toGatewayBase(apiBase: string): string | null {
  try {
    const url = new URL(apiBase);
    const proto = url.protocol === "https:" ? "wss:" : "ws:";
    const path = url.pathname.replace(/\/$/, "");
    const basePath = path.endsWith("/api/v1") ? path.slice(0, -7) : "";
    return `${proto}//${url.host}${basePath}`;
  } catch {
    return null;
  }
}

async function loadServers(
  session: Session,
  apiBase: string,
  setServers: (v: Server[]) => void,
  setError: (v: string | null) => void
) {
  try {
    const res = await fetch(`${apiBase}/servers`, { headers: authHeaders(session) });
    if (!res.ok) throw new Error(`Failed loading servers (${res.status})`);
    const body = await res.json();
    setServers(Array.isArray(body) ? body : []);
  } catch (err) {
    setError((err as Error).message);
  }
}

async function loadChannels(
  session: Session,
  apiBase: string,
  serverId: string,
  setChannels: (v: Channel[]) => void,
  setError: (v: string | null) => void
) {
  try {
    const res = await fetch(`${apiBase}/servers/${serverId}/channels`, {
      headers: authHeaders(session),
    });
    if (!res.ok) throw new Error(`Failed loading channels (${res.status})`);
    const body = await res.json();
    setChannels(Array.isArray(body) ? body : []);
  } catch (err) {
    setError((err as Error).message);
  }
}

async function loadMessages(
  session: Session,
  apiBase: string,
  channelId: string,
  setMessages: (v: Message[]) => void,
  setError: (v: string | null) => void
) {
  try {
    const res = await fetch(`${apiBase}/channels/${channelId}/messages?limit=50`, {
      headers: authHeaders(session),
    });
    if (!res.ok) throw new Error(`Failed loading messages (${res.status})`);
    const body = await res.json();
    setMessages(Array.isArray(body) ? body : []);
  } catch (err) {
    setError((err as Error).message);
  }
}

async function loadExperienceProfile(
  session: Session,
  apiBase: string,
  setMode: (v: ExperienceMode) => void,
  setError: (v: string | null) => void
) {
  try {
    const res = await fetch(`${apiBase}/users/@me/experience-profile`, {
      headers: authHeaders(session),
    });
    if (!res.ok) throw new Error(`Failed loading experience profile (${res.status})`);
    const body = (await res.json()) as ExperienceProfile;
    if (body.experience_mode === "messaging" || body.experience_mode === "full") {
      setMode(body.experience_mode);
    }
  } catch (err) {
    setError((err as Error).message);
  }
}

async function updateExperienceMode(
  session: Session,
  apiBase: string,
  mode: ExperienceMode
) {
  const res = await fetch(`${apiBase}/users/@me/experience-profile`, {
    method: "PUT",
    headers: authHeaders(session),
    body: JSON.stringify({ experience_mode: mode }),
  });
  if (!res.ok) {
    throw new Error(`Failed updating experience mode (${res.status})`);
  }
}

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

    setGatewayStatus("connecting");
    const ws = new WebSocket(`${gatewayBase}/gateway`);

    let heartbeatTimer: number | null = null;

    ws.onopen = () => {
      setGatewayStatus("online");
    };

    ws.onclose = () => {
      setGatewayStatus("offline");
      if (heartbeatTimer) window.clearInterval(heartbeatTimer);
    };

    ws.onerror = () => {
      setGatewayStatus("offline");
    };

    ws.onmessage = (evt) => {
      try {
        const payload = JSON.parse(String(evt.data)) as GatewayEnvelope;

        if (payload.op === "Hello") {
          const heartbeatMs = Number((payload.d as { heartbeat_interval?: number } | undefined)?.heartbeat_interval ?? 45000);
          ws.send(JSON.stringify({
            op: "Identify",
            d: { token: session.accessToken },
          }));

          if (heartbeatTimer) window.clearInterval(heartbeatTimer);
          heartbeatTimer = window.setInterval(() => {
            if (ws.readyState === WebSocket.OPEN) {
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

    return () => {
      if (heartbeatTimer) window.clearInterval(heartbeatTimer);
      ws.close();
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
        {statusText} · Gateway: {gatewayStatus}
      </div>
      {error ? <div className="error" style={{ marginBottom: 8 }}>{error}</div> : null}
      <div className="workspace">
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

        <section className="panel col">
          <h3>Channels</h3>
          {channels.map((c) => (
            <button
              key={c.id}
              className="list-item"
              onClick={() => setActiveChannel(c.id)}
            >
              # {c.name}
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

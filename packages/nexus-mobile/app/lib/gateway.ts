/**
 * gateway.ts - WebSocket gateway client for real-time events
 * Mirrors the pattern from nexus-desktop/src/useGateway.ts
 */
import { api } from "./api";
import { store } from "./store";
import type { Message } from "./api";

export type GatewayEventType =
  | "MESSAGE_CREATE" | "MESSAGE_UPDATE" | "MESSAGE_DELETE"
  | "TYPING_START" | "TYPING_STOP"
  | "PRESENCE_UPDATE" | "CHANNEL_UPDATE" | "CHANNEL_DELETE"
  | "GUILD_UPDATE" | "GUILD_DELETE"
  | "MEMBER_JOIN" | "MEMBER_LEAVE"
  | "READY" | "RESUMED";

type GatewayOpcode =
  | "Hello"
  | "Identify"
  | "Resume"
  | "Dispatch"
  | "Reconnect"
  | "InvalidSession"
  | "Heartbeat"
  | "HeartbeatAck";

export interface GatewayMessage {
  op: GatewayOpcode | number;
  d?: Record<string, unknown>;
  s?: number;
  t?: GatewayEventType;
}

const OP_HELLO = "Hello";
const OP_DISPATCH = "Dispatch";
const OP_HEARTBEAT = "Heartbeat";
const OP_IDENTIFY = "Identify";
const OP_RESUME = "Resume";
const OP_HEARTBEAT_ACK = "HeartbeatAck";
const OP_RECONNECT = "Reconnect";
const OP_INVALID_SESSION = "InvalidSession";

function normalizeOpcode(op: GatewayMessage["op"]): GatewayOpcode {
  if (typeof op === "string") return op as GatewayOpcode;
  switch (op) {
    case 10:
      return "Hello";
    case 0:
      return "Dispatch";
    case 1:
      return "Heartbeat";
    case 2:
      return "Identify";
    case 6:
      return "Resume";
    case 7:
      return "Reconnect";
    case 11:
      return "HeartbeatAck";
    default:
      return "Dispatch";
  }
}

export class GatewayClient {
  private ws: WebSocket | null = null;
  private gatewayUrl: string = "";
  private heartbeatInterval: ReturnType<typeof setInterval> | null = null;
  private heartbeatAcked = true;
  private heartbeatTimer: ReturnType<typeof setTimeout> | null = null;
  private seq = 0;
  private sessionId: string | null = null;
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private intentionalClose = false;
  private handlers: Map<GatewayEventType, Set<(d: Record<string, unknown>) => void>> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;

  async connect(gatewayUrl?: string) {
    if (!gatewayUrl) {
      try { gatewayUrl = await api.getGatewayUrl(); }
      catch { gatewayUrl = "ws://localhost:8081/gateway"; }
    }
    this.gatewayUrl = gatewayUrl;
    this.open();
  }

  private open() {
    this.intentionalClose = false;
    this.ws = new WebSocket(this.gatewayUrl);
    this.ws.onopen = this.onOpen.bind(this);
    this.ws.onmessage = this.onMessage.bind(this);
    this.ws.onerror = this.onError.bind(this);
    this.ws.onclose = this.onClose.bind(this);
  }

  private onOpen() {
    console.log("[Gateway] connected");
    this.reconnectAttempts = 0;
    this.identify();
  }

  private onMessage(event: MessageEvent) {
    try {
      const data: GatewayMessage = JSON.parse(event.data as string);
      this.handlePayload(data);
    } catch (e) {
      console.error("[Gateway] parse error", e);
    }
  }

  private handlePayload(data: GatewayMessage) {
    switch (normalizeOpcode(data.op)) {
      case OP_HELLO:
        this.startHeartbeat(data.d as { heartbeat_interval: number });
        break;
      case OP_DISPATCH:
        this.seq = data.s ?? this.seq;
        if (data.t) this.dispatch(data.t, data.d ?? {});
        break;
      case OP_HEARTBEAT_ACK:
        this.heartbeatAcked = true;
        break;
      case OP_RECONNECT:
        this.ws?.close();
        break;
      case OP_INVALID_SESSION:
        this.ws?.close();
        break;
    }
  }

  private startHeartbeat(d: { heartbeat_interval: number }) {
    const interval = d.heartbeat_interval;
    if (this.heartbeatInterval) clearInterval(this.heartbeatInterval);
    this.heartbeatInterval = setInterval(() => {
      if (!this.heartbeatAcked) {
        console.warn("[Gateway] heartbeat timeout, reconnecting...");
        this.ws?.close(); return;
      }
      this.heartbeatAcked = false;
      this.send({ op: OP_HEARTBEAT, d: { timestamp: Date.now() } });
    }, interval);
  }

  private identify() {
    if (!store.session) return;
    this.send({
      op: OP_IDENTIFY,
      d: { token: store.session.accessToken },
    });
  }

  private send(data: GatewayMessage) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data));
    }
  }

  private dispatch(type: GatewayEventType, d: Record<string, unknown>) {
    // Built-in handlers
    switch (type) {
      case "READY": {
        const sessionData = d as { session_id: string };
        this.sessionId = sessionData.session_id;
        console.log("[Gateway] ready, session:", this.sessionId);
        break;
      }
      case "MESSAGE_CREATE": {
        const msg = d as unknown as Message;
        if (msg.authorId !== store.session?.user.id) {
          store.appendMessage(msg);
        }
        break;
      }
      case "MESSAGE_DELETE": {
        const del = d as { id: string; channel_id: string };
        const msgs = store.messages[del.channel_id];
        if (msgs) {
          store.messages = {
            ...store.messages,
            [del.channel_id]: msgs.filter(m => m.id !== del.id),
          };
        }
        break;
      }
      case "TYPING_START": {
        const typing = d as { channel_id: string; username: string };
        if (store.activeChannelId === typing.channel_id) {
          const current = store.typingUsers[typing.channel_id] ?? [];
          if (!current.includes(typing.username)) {
            store.typingUsers = {
              ...store.typingUsers,
              [typing.channel_id]: [...current, typing.username],
            };
          }
          // Auto-clear after 5s
          setTimeout(() => {
            const users = store.typingUsers[typing.channel_id] ?? [];
            if (users.includes(typing.username)) {
              store.typingUsers = {
                ...store.typingUsers,
                [typing.channel_id]: users.filter(u => u !== typing.username),
              };
            }
          }, 5000);
        }
        break;
      }
    }

    // Custom handlers
    const handlers = this.handlers.get(type);
    if (handlers) handlers.forEach(fn => fn(d));
  }

  on(event: GatewayEventType, fn: (d: Record<string, unknown>) => void) {
    if (!this.handlers.has(event)) this.handlers.set(event, new Set());
    this.handlers.get(event)!.add(fn);
    return () => this.handlers.get(event)?.delete(fn);
  }

  private onError(e: Event) {
    console.error("[Gateway] error", e);
  }

  private onClose(e: CloseEvent) {
    console.warn("[Gateway] closed", e.code, e.reason);
    if (this.heartbeatInterval) clearInterval(this.heartbeatInterval);
    if (this.heartbeatTimer) clearTimeout(this.heartbeatTimer);
    if (!this.intentionalClose && this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
      console.log("[Gateway] reconnecting in", delay, "ms (attempt", this.reconnectAttempts, ")");
      this.reconnectTimeout = setTimeout(() => this.open(), delay);
    }
  }

  sendTyping(channelId: string) {
    // Typing indicators go via REST API, not WebSocket
    api.sendTyping(channelId).catch(() => {});
  }

  get status(): string {
    if (!this.ws) return "disconnected";
    return this.ws.readyState === WebSocket.OPEN ? "connected" :
           this.ws.readyState === WebSocket.CONNECTING ? "connecting" : "disconnected";
  }

  disconnect() {
    this.intentionalClose = true;
    if (this.reconnectTimeout) clearTimeout(this.reconnectTimeout);
    if (this.heartbeatInterval) clearInterval(this.heartbeatInterval);
    if (this.heartbeatTimer) clearTimeout(this.heartbeatTimer);
    this.ws?.close();
  }
}

export const gateway = new GatewayClient();

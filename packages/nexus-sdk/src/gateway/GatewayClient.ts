import { EventEmitter } from "events";
import WebSocket from "ws";
import type { Interaction } from "../types/index.js";

// Gateway opcodes (mirrors the Rust gateway implementation)
export const GatewayOp = {
  Dispatch: 0,
  Heartbeat: 1,
  Identify: 2,
  Resume: 6,
  Reconnect: 7,
  HeartbeatAck: 11,
} as const;

export interface GatewayClientOptions {
  /** WebSocket URL, e.g. "ws://localhost:3001" */
  gatewayUrl: string;
  /** Bot token: "Bot <raw>" */
  token: string;
  /** Heartbeat interval in ms (default 30 000). Server may override. */
  heartbeatInterval?: number;
  /** Auto-reconnect on disconnect (default true). */
  reconnect?: boolean;
  /** Maximum reconnect attempts before giving up (default 10). */
  maxReconnects?: number;
}

export interface GatewayClientEvents {
  ready: [data: { session_id: string; user: unknown }];
  dispatch: [event: string, data: unknown];
  interaction: [interaction: Interaction];
  close: [code: number, reason: string];
  error: [error: Error];
  reconnecting: [attempt: number];
}

/**
 * WebSocket gateway client for Nexus bots.
 *
 * ```ts
 * const gateway = new GatewayClient({ gatewayUrl, token });
 * gateway.on("interaction", (interaction) => { ... });
 * await gateway.connect();
 * ```
 */
export class GatewayClient extends EventEmitter {
  private ws: WebSocket | null = null;
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private lastSeq: number | null = null;
  private sessionId: string | null = null;
  private reconnectAttempt = 0;
  private destroyed = false;

  private readonly gatewayUrl: string;
  private readonly token: string;
  private readonly heartbeatInterval: number;
  private readonly reconnect: boolean;
  private readonly maxReconnects: number;

  constructor(opts: GatewayClientOptions) {
    super();
    this.gatewayUrl = opts.gatewayUrl;
    this.token = opts.token.startsWith("Bot ") ? opts.token : `Bot ${opts.token}`;
    this.heartbeatInterval = opts.heartbeatInterval ?? 30_000;
    this.reconnect = opts.reconnect ?? true;
    this.maxReconnects = opts.maxReconnects ?? 10;
  }

  // --------------------------------------------------------------------------
  // Lifecycle
  // --------------------------------------------------------------------------

  /** Open the gateway connection. Resolves once a Ready event is received. */
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.once("ready", () => resolve());
      this.once("error", reject);
      this._open();
    });
  }

  /** Close the connection gracefully and stop reconnecting. */
  destroy(): void {
    this.destroyed = true;
    this._stopHeartbeat();
    if (this.ws) {
      this.ws.close(1000, "Client destroyed");
      this.ws = null;
    }
  }

  // --------------------------------------------------------------------------
  // Internal connection management
  // --------------------------------------------------------------------------

  private _open(): void {
    const ws = new WebSocket(this.gatewayUrl, {
      headers: { Authorization: this.token },
    });

    this.ws = ws;

    ws.on("open", () => {
      this.reconnectAttempt = 0;
      if (this.sessionId) {
        // Resume a disconnected session
        this._send(GatewayOp.Resume, {
          token: this.token,
          session_id: this.sessionId,
          seq: this.lastSeq,
        });
      } else {
        // Fresh identify
        this._send(GatewayOp.Identify, {
          token: this.token,
          properties: {
            os: process.platform,
            browser: "@nexus/sdk",
            device: "@nexus/sdk",
          },
          intents: 0, // bots can specify intents later
        });
      }
      this._startHeartbeat();
    });

    ws.on("message", (raw: WebSocket.RawData) => {
      let payload: { op: number; t: string | null; d: unknown; s: number | null };
      try {
        payload = JSON.parse(raw.toString());
      } catch {
        return;
      }

      if (payload.s !== null && payload.s !== undefined) {
        this.lastSeq = payload.s;
      }

      switch (payload.op) {
        case GatewayOp.Dispatch:
          this._handleDispatch(payload.t!, payload.d);
          break;

        case GatewayOp.HeartbeatAck:
          // Confirmed alive
          break;

        case GatewayOp.Heartbeat:
          // Server-requested heartbeat
          this._sendHeartbeat();
          break;

        case GatewayOp.Reconnect:
          ws.close(4000, "Server requested reconnect");
          break;
      }
    });

    ws.on("close", (code, reason) => {
      this._stopHeartbeat();

      const reasonStr = reason.toString();
      this.emit("close", code, reasonStr);

      // 1000 = clean close (destroy() was called)
      if (!this.destroyed && this.reconnect && code !== 1000) {
        this._scheduleReconnect();
      } else {
        this.ws = null;
      }
    });

    ws.on("error", (err: Error) => {
      this.emit("error", err);
    });
  }

  private _handleDispatch(event: string, data: unknown): void {
    if (event === "READY") {
      const d = data as { session_id: string; user: unknown };
      this.sessionId = d.session_id;
      this.emit("ready", d);
    }

    this.emit("dispatch", event, data);

    if (event === "INTERACTION_CREATE") {
      this.emit("interaction", data as Interaction);
    }
  }

  // --------------------------------------------------------------------------
  // Heartbeat
  // --------------------------------------------------------------------------

  private _startHeartbeat(): void {
    this._stopHeartbeat();
    this.heartbeatTimer = setInterval(
      () => this._sendHeartbeat(),
      this.heartbeatInterval
    );
  }

  private _stopHeartbeat(): void {
    if (this.heartbeatTimer !== null) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private _sendHeartbeat(): void {
    this._send(GatewayOp.Heartbeat, this.lastSeq);
  }

  // --------------------------------------------------------------------------
  // Helpers
  // --------------------------------------------------------------------------

  private _send(op: number, d: unknown): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ op, d }));
    }
  }

  private _scheduleReconnect(): void {
    this.reconnectAttempt++;
    if (this.reconnectAttempt > this.maxReconnects) {
      const err = new Error(
        `Gateway: exceeded max reconnect attempts (${this.maxReconnects})`
      );
      this.emit("error", err);
      return;
    }

    this.emit("reconnecting", this.reconnectAttempt);

    const delay = Math.min(1000 * 2 ** this.reconnectAttempt, 30_000);
    setTimeout(() => this._open(), delay);
  }

  // --------------------------------------------------------------------------
  // Typed EventEmitter overloads
  // --------------------------------------------------------------------------

  override on<K extends keyof GatewayClientEvents>(
    event: K,
    listener: (...args: GatewayClientEvents[K]) => void
  ): this;
  override on(event: string, listener: (...args: unknown[]) => void): this;
  override on(event: string, listener: (...args: unknown[]) => void): this {
    return super.on(event, listener);
  }

  override once<K extends keyof GatewayClientEvents>(
    event: K,
    listener: (...args: GatewayClientEvents[K]) => void
  ): this;
  override once(event: string, listener: (...args: unknown[]) => void): this;
  override once(event: string, listener: (...args: unknown[]) => void): this {
    return super.once(event, listener);
  }

  override emit<K extends keyof GatewayClientEvents>(
    event: K,
    ...args: GatewayClientEvents[K]
  ): boolean;
  override emit(event: string, ...args: unknown[]): boolean;
  override emit(event: string, ...args: unknown[]): boolean {
    return super.emit(event, ...args);
  }
}

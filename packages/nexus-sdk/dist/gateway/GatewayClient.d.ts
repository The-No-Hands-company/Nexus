import { EventEmitter } from "events";
import type { Interaction } from "../types/index.js";
export declare const GatewayOp: {
    readonly Dispatch: 0;
    readonly Heartbeat: 1;
    readonly Identify: 2;
    readonly Resume: 6;
    readonly Reconnect: 7;
    readonly HeartbeatAck: 11;
};
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
    ready: [data: {
        session_id: string;
        user: unknown;
    }];
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
export declare class GatewayClient extends EventEmitter {
    private ws;
    private heartbeatTimer;
    private lastSeq;
    private sessionId;
    private reconnectAttempt;
    private destroyed;
    private readonly gatewayUrl;
    private readonly token;
    private readonly heartbeatInterval;
    private readonly reconnect;
    private readonly maxReconnects;
    constructor(opts: GatewayClientOptions);
    /** Open the gateway connection. Resolves once a Ready event is received. */
    connect(): Promise<void>;
    /** Close the connection gracefully and stop reconnecting. */
    destroy(): void;
    private _open;
    private _handleDispatch;
    private _startHeartbeat;
    private _stopHeartbeat;
    private _sendHeartbeat;
    private _send;
    private _scheduleReconnect;
    on<K extends keyof GatewayClientEvents>(event: K, listener: (...args: GatewayClientEvents[K]) => void): this;
    on(event: string, listener: (...args: unknown[]) => void): this;
    once<K extends keyof GatewayClientEvents>(event: K, listener: (...args: GatewayClientEvents[K]) => void): this;
    once(event: string, listener: (...args: unknown[]) => void): this;
    emit<K extends keyof GatewayClientEvents>(event: K, ...args: GatewayClientEvents[K]): boolean;
    emit(event: string, ...args: unknown[]): boolean;
}
//# sourceMappingURL=GatewayClient.d.ts.map
"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.GatewayClient = exports.GatewayOp = void 0;
const events_1 = require("events");
const ws_1 = __importDefault(require("ws"));
// Gateway opcodes (mirrors the Rust gateway implementation)
exports.GatewayOp = {
    Dispatch: 0,
    Heartbeat: 1,
    Identify: 2,
    Resume: 6,
    Reconnect: 7,
    HeartbeatAck: 11,
};
/**
 * WebSocket gateway client for Nexus bots.
 *
 * ```ts
 * const gateway = new GatewayClient({ gatewayUrl, token });
 * gateway.on("interaction", (interaction) => { ... });
 * await gateway.connect();
 * ```
 */
class GatewayClient extends events_1.EventEmitter {
    ws = null;
    heartbeatTimer = null;
    lastSeq = null;
    sessionId = null;
    reconnectAttempt = 0;
    destroyed = false;
    gatewayUrl;
    token;
    heartbeatInterval;
    reconnect;
    maxReconnects;
    constructor(opts) {
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
    connect() {
        return new Promise((resolve, reject) => {
            this.once("ready", () => resolve());
            this.once("error", reject);
            this._open();
        });
    }
    /** Close the connection gracefully and stop reconnecting. */
    destroy() {
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
    _open() {
        const ws = new ws_1.default(this.gatewayUrl, {
            headers: { Authorization: this.token },
        });
        this.ws = ws;
        ws.on("open", () => {
            this.reconnectAttempt = 0;
            if (this.sessionId) {
                // Resume a disconnected session
                this._send(exports.GatewayOp.Resume, {
                    token: this.token,
                    session_id: this.sessionId,
                    seq: this.lastSeq,
                });
            }
            else {
                // Fresh identify
                this._send(exports.GatewayOp.Identify, {
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
        ws.on("message", (raw) => {
            let payload;
            try {
                payload = JSON.parse(raw.toString());
            }
            catch {
                return;
            }
            if (payload.s !== null && payload.s !== undefined) {
                this.lastSeq = payload.s;
            }
            switch (payload.op) {
                case exports.GatewayOp.Dispatch:
                    this._handleDispatch(payload.t, payload.d);
                    break;
                case exports.GatewayOp.HeartbeatAck:
                    // Confirmed alive
                    break;
                case exports.GatewayOp.Heartbeat:
                    // Server-requested heartbeat
                    this._sendHeartbeat();
                    break;
                case exports.GatewayOp.Reconnect:
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
            }
            else {
                this.ws = null;
            }
        });
        ws.on("error", (err) => {
            this.emit("error", err);
        });
    }
    _handleDispatch(event, data) {
        if (event === "READY") {
            const d = data;
            this.sessionId = d.session_id;
            this.emit("ready", d);
        }
        this.emit("dispatch", event, data);
        if (event === "INTERACTION_CREATE") {
            this.emit("interaction", data);
        }
    }
    // --------------------------------------------------------------------------
    // Heartbeat
    // --------------------------------------------------------------------------
    _startHeartbeat() {
        this._stopHeartbeat();
        this.heartbeatTimer = setInterval(() => this._sendHeartbeat(), this.heartbeatInterval);
    }
    _stopHeartbeat() {
        if (this.heartbeatTimer !== null) {
            clearInterval(this.heartbeatTimer);
            this.heartbeatTimer = null;
        }
    }
    _sendHeartbeat() {
        this._send(exports.GatewayOp.Heartbeat, this.lastSeq);
    }
    // --------------------------------------------------------------------------
    // Helpers
    // --------------------------------------------------------------------------
    _send(op, d) {
        if (this.ws?.readyState === ws_1.default.OPEN) {
            this.ws.send(JSON.stringify({ op, d }));
        }
    }
    _scheduleReconnect() {
        this.reconnectAttempt++;
        if (this.reconnectAttempt > this.maxReconnects) {
            const err = new Error(`Gateway: exceeded max reconnect attempts (${this.maxReconnects})`);
            this.emit("error", err);
            return;
        }
        this.emit("reconnecting", this.reconnectAttempt);
        const delay = Math.min(1000 * 2 ** this.reconnectAttempt, 30_000);
        setTimeout(() => this._open(), delay);
    }
    on(event, listener) {
        return super.on(event, listener);
    }
    once(event, listener) {
        return super.once(event, listener);
    }
    emit(event, ...args) {
        return super.emit(event, ...args);
    }
}
exports.GatewayClient = GatewayClient;
//# sourceMappingURL=GatewayClient.js.map
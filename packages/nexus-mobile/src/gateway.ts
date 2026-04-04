// ── Nexus Mobile — Gateway WebSocket hook ────────────────────────────────────
//
// Mirrors the nexus-web gateway.ts behaviour:
// - Connects on mount when a session is present
// - Sends Identify + periodic Heartbeat
// - Dispatches incoming events to the store
// - Exponential backoff reconnect (1s → 30s cap)
// - Cleans up fully on unmount / session change

import { useEffect, useRef } from "react";
import { useStore } from "./store";
import type { NxMessage, NxUser } from "./types";

export function useGateway() {
  const {
    session,
    setGatewayStatus,
    onMessageCreate,
    onMessageDelete,
    onPresenceUpdate,
    onTypingStart,
  } = useStore();

  const wsRef = useRef<WebSocket | null>(null);
  const heartbeatRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const reconnectRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const attemptsRef = useRef(0);
  const stoppedRef = useRef(false);

  useEffect(() => {
    if (!session) {
      setGatewayStatus("offline");
      return;
    }

    stoppedRef.current = false;

    // Convert http(s) server URL to ws(s) for the gateway endpoint
    const wsBase = session.serverUrl
      .replace(/\/+$/, "")
      .replace(/^http/, "ws");

    const connect = () => {
      if (stoppedRef.current) return;
      setGatewayStatus("connecting");

      const ws = new WebSocket(`${wsBase}/gateway`);
      wsRef.current = ws;

      ws.onopen = () => {
        attemptsRef.current = 0;
      };

      ws.onclose = () => {
        setGatewayStatus("offline");
        if (heartbeatRef.current) clearInterval(heartbeatRef.current);
        if (stoppedRef.current) return;
        attemptsRef.current += 1;
        // Exponential backoff capped at 30 seconds
        const delay = Math.min(30_000, 1_000 * 2 ** Math.min(attemptsRef.current, 5));
        reconnectRef.current = setTimeout(connect, delay);
      };

      ws.onerror = (err) => {
        console.warn("[gateway] WebSocket error:", err);
        // onclose will fire after onerror — reconnect logic is there
      };

      ws.onmessage = (evt) => {
        try {
          const payload = JSON.parse(
            typeof evt.data === "string" ? evt.data : ""
          ) as { op: string; d?: unknown };

          switch (payload.op) {
            case "Hello": {
              const interval =
                (payload.d as { heartbeat_interval?: number })
                  ?.heartbeat_interval ?? 45_000;

              // Identify immediately
              ws.send(
                JSON.stringify({
                  op: "Identify",
                  d: { token: session.accessToken },
                })
              );

              // Start heartbeat
              if (heartbeatRef.current) clearInterval(heartbeatRef.current);
              heartbeatRef.current = setInterval(() => {
                if (ws.readyState === WebSocket.OPEN) {
                  ws.send(
                    JSON.stringify({ op: "Heartbeat", d: { timestamp: Date.now() } })
                  );
                }
              }, Math.max(interval, 5_000));
              break;
            }

            case "Ready":
              setGatewayStatus("online");
              break;

            case "InvalidSession":
              setGatewayStatus("offline");
              ws.close();
              break;

            case "Dispatch": {
              const d = payload.d as { event?: string; data?: unknown };
              if (!d?.event) break;

              switch (d.event) {
                case "MESSAGE_CREATE":
                  onMessageCreate(d.data as NxMessage);
                  break;

                case "MESSAGE_DELETE": {
                  const md = d.data as { message_id: string; channel_id: string };
                  onMessageDelete(md.channel_id, md.message_id);
                  break;
                }

                case "PRESENCE_UPDATE": {
                  const pd = d.data as { user_id: string; status: string };
                  onPresenceUpdate(pd.user_id, pd.status as NxUser["presence"]);
                  break;
                }

                case "TYPING_START": {
                  const td = d.data as { channel_id: string; username: string };
                  onTypingStart(td.channel_id, td.username);
                  break;
                }
              }
              break;
            }
          }
        } catch {
          // Silently ignore malformed frames
        }
      };
    };

    connect();

    return () => {
      stoppedRef.current = true;
      if (heartbeatRef.current) clearInterval(heartbeatRef.current);
      if (reconnectRef.current) clearTimeout(reconnectRef.current);
      wsRef.current?.close();
      setGatewayStatus("offline");
    };
  }, [session?.accessToken]);
}

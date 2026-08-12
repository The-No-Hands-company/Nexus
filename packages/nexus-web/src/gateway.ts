import { useEffect, useRef } from "react";
import { useStore, apiBase, NxMessage } from "./store";

export function useGateway() {
  const { session, setGatewayStatus, onMessageCreate, onMessageDelete, onPresenceUpdate, onTypingStart } = useStore();
  const wsRef = useRef<WebSocket | null>(null);
  const timerRef = useRef<number | null>(null);
  const reconnRef = useRef<number | null>(null);
  const attemptsRef = useRef(0);
  const stoppedRef = useRef(false);

  useEffect(() => {
    if (!session) {
      setGatewayStatus("offline");
      return;
    }

    stoppedRef.current = false;

    const base = apiBase(session)
      .replace(/\/api\/v1$/, "")
      .replace(/^http/, "ws");

    const connect = () => {
      if (stoppedRef.current) return;
      setGatewayStatus("connecting");

      const ws = new WebSocket(`${base}/gateway`);
      wsRef.current = ws;

      ws.onopen = () => { attemptsRef.current = 0; };

      ws.onclose = () => {
        setGatewayStatus("offline");
        if (stoppedRef.current) return;
        attemptsRef.current += 1;
        const delay = Math.min(30_000, 1_000 * 2 ** Math.min(attemptsRef.current, 5));
        reconnRef.current = window.setTimeout(connect, delay);
      };

      ws.onmessage = (evt) => {
        try {
          const payload = JSON.parse(String(evt.data)) as {
            op: string;
            d?: unknown;
          };

          if (payload.op === "Hello") {
            const interval =
              (payload.d as { heartbeat_interval?: number })?.heartbeat_interval ?? 45_000;
            // No credential. The browser sent its session cookie with the
            // handshake, the proxy exchanged it for a signed identity token and
            // put that on the upstream connection, and the server had already
            // verified it before this socket existed. There is nothing left for
            // the client to prove, and it holds no token to prove it with.
            ws.send(JSON.stringify({ op: "Identify", d: {} }));
            timerRef.current = window.setInterval(() => {
              if (ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({ op: "Heartbeat", d: { timestamp: Date.now() } }));
              }
            }, Math.max(interval, 5_000));
          }

          if (payload.op === "Ready") {
            setGatewayStatus("online");
          }

          if (payload.op === "InvalidSession") {
            setGatewayStatus("offline");
            ws.close();
          }

          if (payload.op === "Dispatch") {
            const d = payload.d as { event?: string; data?: unknown };
            if (!d?.event) return;

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
                onPresenceUpdate(pd.user_id, pd.status as "online");
                break;
              }
              case "TYPING_START": {
                const td = d.data as { channel_id: string; username: string };
                onTypingStart(td.channel_id, td.username);
                break;
              }
            }
          }
        } catch { /* ignore malformed */ }
      };
    };

    connect();

    return () => {
      stoppedRef.current = true;
      if (timerRef.current) window.clearInterval(timerRef.current);
      if (reconnRef.current) window.clearTimeout(reconnRef.current);
      wsRef.current?.close();
      setGatewayStatus("offline");
    };
  }, [session?.accessToken]);
}

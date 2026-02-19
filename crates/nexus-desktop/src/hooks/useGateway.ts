/**
 * useGateway — connects to the Nexus WebSocket gateway and dispatches
 * incoming events into the Zustand store.
 */
import { useEffect, useRef } from "react";
import { useStore, Message, VoiceParticipant } from "../store";

interface GatewayEvent {
  event_type: string;
  data: unknown;
  server_id?: string;
  channel_id?: string;
  user_id?: string;
}

export function useGateway() {
  const { session, appendMessage, setVoiceParticipants, setPttActive } =
    useStore();
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!session) return;

    const connect = () => {
      const wsUrl = session.serverUrl
        .replace(/^http/, "ws")
        .replace(/\/$/, "");
      const ws = new WebSocket(
        `${wsUrl}/ws/gateway?token=${session.accessToken}`
      );
      wsRef.current = ws;

      ws.onopen = () => {
        console.log("[gateway] connected");
      };

      ws.onmessage = (ev) => {
        let event: GatewayEvent;
        try {
          event = JSON.parse(ev.data as string) as GatewayEvent;
        } catch {
          return;
        }
        handleEvent(event);
      };

      ws.onclose = () => {
        console.log("[gateway] closed, reconnecting in 3s");
        reconnectTimer.current = setTimeout(connect, 3000);
      };

      ws.onerror = (err) => {
        console.error("[gateway] error", err);
        ws.close();
      };
    };

    const handleEvent = (event: GatewayEvent) => {
      switch (event.event_type) {
        case "MESSAGE_CREATE": {
          const raw = event.data as {
            id: string;
            channel_id: string;
            author_id: string;
            author_username?: string;
            content: string;
            created_at: string;
            edited_at?: string;
          };
          const msg: Message = {
            id: raw.id,
            channelId: raw.channel_id,
            authorId: raw.author_id,
            authorUsername: raw.author_username ?? "Unknown",
            content: raw.content,
            createdAt: raw.created_at,
            editedAt: raw.edited_at,
          };
          appendMessage(msg.channelId, msg);
          break;
        }
        case "VOICE_STATE_UPDATE": {
          const participants = event.data as VoiceParticipant[];
          setVoiceParticipants(participants);
          break;
        }
        case "PTT_START": {
          setPttActive(true);
          break;
        }
        case "PTT_STOP": {
          setPttActive(false);
          break;
        }
        default:
          break;
      }
    };

    connect();

    return () => {
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      wsRef.current?.close();
    };
  }, [session, appendMessage, setVoiceParticipants, setPttActive]);
}

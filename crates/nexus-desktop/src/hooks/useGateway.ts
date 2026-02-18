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
        case "message_create": {
          const msg = event.data as Message;
          appendMessage(msg.channelId, msg);
          break;
        }
        case "voice_state_update": {
          const participants = event.data as VoiceParticipant[];
          setVoiceParticipants(participants);
          break;
        }
        case "ptt_start": {
          setPttActive(true);
          break;
        }
        case "ptt_stop": {
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

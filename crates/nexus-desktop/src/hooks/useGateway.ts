/**
 * useGateway — connects to the Nexus WebSocket gateway and dispatches
 * incoming events into the Zustand store.
 *
 * Protocol:
 *   1. Connect to ws://host:8081/gateway (no token in URL)
 *   2. Server sends {"op":"Hello","d":{"heartbeat_interval":45000}}
 *   3. Client sends {"op":"Identify","d":{"token":"<jwt>"}}
 *   4. Server sends {"op":"Ready","d":{...}}
 *   5. Events arrive as {"op":"Dispatch","d":{"event":"EVENT_NAME","data":{...}}}
 */
import { useEffect, useRef } from "react";
import { useStore, Message, VoiceParticipant } from "../store";
import { isTauri } from "../invoke";

let _sendNotification: ((title: string, body: string) => void) | null = null;

// Lazily import the Tauri notification plugin — no-op in browser mode.
async function sendOsNotification(title: string, body: string) {
  if (!isTauri()) return;
  try {
    if (!_sendNotification) {
      const mod = await import("@tauri-apps/plugin-notification");
      _sendNotification = (t, b) => mod.sendNotification({ title: t, body: b });
    }
    _sendNotification(title, body);
  } catch {
    // plugin not available — silently skip
  }
}

interface WireMessage {
  op: string;
  d: unknown;
}

export function useGateway() {
  const { session, appendMessage, setVoiceParticipants, setPttActive, setTyping, updateMessageReaction, addInAppNotification } =
    useStore();
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const heartbeatTimer = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (!session) return;

    // Prevents onclose from scheduling a reconnect after intentional cleanup
    let destroyed = false;

    const connect = () => {
      if (destroyed) return;
      // Gateway runs on port 8081 with path /gateway.
      // Replace port 8080 (API) with 8081, or append :8081 if no port present.
      const wsBase = session.serverUrl
        .replace(/^http/, "ws")
        .replace(/\/$/, "");
      const wsUrl = wsBase.includes(":8080")
        ? wsBase.replace(":8080", ":8081")
        : wsBase.replace(/(:\d+)?$/, ":8081");

      const ws = new WebSocket(`${wsUrl}/gateway`);
      wsRef.current = ws;

      ws.onopen = () => {
        console.log("[gateway] connected — sending Identify");
        ws.send(
          JSON.stringify({ op: "Identify", d: { token: session.accessToken } })
        );
      };

      ws.onmessage = (ev) => {
        let wire: WireMessage;
        try {
          wire = JSON.parse(ev.data as string) as WireMessage;
        } catch {
          return;
        }

        switch (wire.op) {
          case "Hello": {
            // Server requests heartbeats — start sending them
            const interval =
              (wire.d as { heartbeat_interval: number }).heartbeat_interval ??
              45000;
            if (heartbeatTimer.current) clearInterval(heartbeatTimer.current);
            heartbeatTimer.current = setInterval(() => {
              if (ws.readyState === WebSocket.OPEN) {
                ws.send(
                  JSON.stringify({
                    op: "Heartbeat",
                    d: { timestamp: Date.now() },
                  })
                );
              }
            }, interval);
            break;
          }

          case "Ready":
            console.log("[gateway] READY received");
            break;

          case "Dispatch": {
            const dispatch = wire.d as { event: string; data: unknown };
            handleEvent(dispatch.event, dispatch.data);
            break;
          }

          case "InvalidSession":
            console.warn("[gateway] InvalidSession — will reconnect");
            ws.close();
            break;

          default:
            break;
        }
      };

      ws.onclose = () => {
        if (destroyed) return; // intentional cleanup — don't reconnect
        console.log("[gateway] closed, reconnecting in 3s");
        if (heartbeatTimer.current) clearInterval(heartbeatTimer.current);
        reconnectTimer.current = setTimeout(connect, 3000);
      };

      ws.onerror = (err) => {
        console.error("[gateway] error", err);
        ws.close();
      };
    };

    const handleEvent = (eventType: string, data: unknown) => {
      switch (eventType) {
        case "MESSAGE_CREATE": {
          const raw = data as {
            id: string;
            channel_id: string;
            author_id: string;
            author_username?: string;
            content: string;
            created_at: string;
            edited_at?: string;
            reactions?: { emoji: string; count: number; me: boolean }[];
            embeds?: object[];
            thread_id?: string;
          };

          // Dedup: skip if this message was already added optimistically
          const existing =
            useStore.getState().messages[raw.channel_id] ?? [];
          if (existing.some((m) => m.id === raw.id)) break;

          const msg: Message = {
            id: raw.id,
            channelId: raw.channel_id,
            authorId: raw.author_id,
            authorUsername: raw.author_username ?? "Unknown",
            content: raw.content,
            createdAt: raw.created_at,
            editedAt: raw.edited_at,
            reactions: raw.reactions ?? [],
            embeds: raw.embeds ?? [],
            threadId: raw.thread_id,
          };
          appendMessage(msg.channelId, msg);

          // In-app + OS notification for @mentions
          const currentUserId = useStore.getState().session?.userId;
          const currentUsername = useStore.getState().session?.username ?? "";
          const isMention =
            currentUserId != null &&
            (raw.author_id !== currentUserId) &&
            (raw.content.includes(`@${currentUsername}`) || raw.content.includes(`@everyone`));
          if (isMention) {
            // Always push to the in-app notification tray
            const channelName = useStore.getState().channels.find(
              (c) => c.id === raw.channel_id
            )?.name;
            addInAppNotification({
              id: raw.id,
              channelId: raw.channel_id,
              channelName,
              authorId: raw.author_id,
              authorUsername: raw.author_username ?? "Unknown",
              content: raw.content,
              createdAt: raw.created_at,
            });
            // Also fire OS notification when window is hidden
            if (document.hidden) {
              sendOsNotification(
                `${raw.author_username ?? "Someone"} mentioned you`,
                raw.content.length > 120 ? raw.content.slice(0, 120) + "…" : raw.content
              );
            }
          }
          break;
        }

        case "TYPING_START": {
          const raw = data as {
            channel_id: string;
            user_id: string;
            username?: string;
          };
          setTyping(raw.channel_id, raw.username ?? raw.user_id, true);
          break;
        }

        case "VOICE_STATE_UPDATE": {
          const participants = data as VoiceParticipant[];
          setVoiceParticipants(participants);
          break;
        }

        case "MESSAGE_REACTION_ADD": {
          const raw = data as {
            message_id: string;
            channel_id: string;
            user_id: string;
            emoji: string;
          };
          const mine = raw.user_id === useStore.getState().session?.userId;
          updateMessageReaction(raw.channel_id, raw.message_id, raw.emoji, +1, mine);
          break;
        }

        case "MESSAGE_REACTION_REMOVE": {
          const raw = data as {
            message_id: string;
            channel_id: string;
            user_id: string;
            emoji: string;
          };
          const mine = raw.user_id === useStore.getState().session?.userId;
          updateMessageReaction(raw.channel_id, raw.message_id, raw.emoji, -1, mine);
          break;
        }

        case "PTT_START":
          setPttActive(true);
          break;

        case "PTT_STOP":
          setPttActive(false);
          break;

        default:
          break;
      }
    };

    connect();

    return () => {
      destroyed = true;
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      if (heartbeatTimer.current) clearInterval(heartbeatTimer.current);
      wsRef.current?.close();
    };
  }, [session, appendMessage, setVoiceParticipants, setPttActive, setTyping, updateMessageReaction, addInAppNotification]);
}

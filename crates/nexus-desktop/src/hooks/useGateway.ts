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
import { useStore, Message, VoiceParticipant, UserBadge, CanvasBlock, ServerEvent } from "../store";
import { isTauri, invoke } from "../invoke";

let _sendNotification: ((title: string, body: string) => void) | null = null;

async function sendOsNotification(title: string, body: string) {
  if (!isTauri()) return;
  try {
    if (!_sendNotification) {
      const mod = await import("@tauri-apps/plugin-notification");
      _sendNotification = (t, b) => mod.sendNotification({ title: t, body: b });
    }
    _sendNotification(title, body);
  } catch {
  }
}

interface WireMessage {
  op: string;
  d: unknown;
}

export function useGateway() {
  const {
    session,
    appendMessage,
    setVoiceParticipants,
    setPttActive,
    setTyping,
    updateMessageReaction,
    addInAppNotification,
    setGatewayStatus,
    onMessageDelete,
  } = useStore();
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const heartbeatTimer = useRef<ReturnType<typeof setInterval> | null>(null);
  const heartbeatTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttempts = useRef(0);
  const closingIntentional = useRef(false);

  useEffect(() => {
    if (!session) {
      setGatewayStatus("offline");
      return;
    }

    closingIntentional.current = false;

    const connect = () => {
      if (closingIntentional.current) return;
      setGatewayStatus("connecting");

      const wsBase = session.serverUrl
        .replace(/^http/, "ws")
        .replace(/\/$/, "");
      const wsUrl = wsBase.includes(":8080")
        ? wsBase.replace(":8080", ":8081")
        : wsBase.replace(/(:\d+)?$/, ":8081");

      const ws = new WebSocket(`${wsUrl}/gateway`);
      wsRef.current = ws;

      ws.onopen = () => {
        reconnectAttempts.current = 0;
      };

      ws.onmessage = (ev) => {
        let wire: WireMessage;
        try {
          wire = JSON.parse(ev.data as string) as WireMessage;
        } catch {
          return;
        }

        if (wire.op === "Hello") {
          const interval = (wire.d as { heartbeat_interval?: number })?.heartbeat_interval ?? 45_000;
          if (heartbeatTimer.current) clearInterval(heartbeatTimer.current);
          if (heartbeatTimeout.current) clearTimeout(heartbeatTimeout.current);

          ws.send(JSON.stringify({ op: "Identify", d: { token: session.accessToken } }));
          heartbeatTimer.current = setInterval(() => {
            if (ws.readyState !== WebSocket.OPEN) return;
            ws.send(JSON.stringify({ op: "Heartbeat", d: { timestamp: Date.now() } }));
            if (heartbeatTimeout.current) clearTimeout(heartbeatTimeout.current);
            heartbeatTimeout.current = setTimeout(() => {
              ws.close();
            }, Math.max(interval * 2, 10_000));
          }, Math.max(interval, 5_000));
          return;
        }

        if (wire.op === "Ready") {
          setGatewayStatus("online");
          return;
        }

        if (wire.op === "InvalidSession") {
          setGatewayStatus("offline");
          ws.close();
          return;
        }

        if (wire.op !== "Dispatch") return;

        const dispatch = wire.d as { event?: string; data?: unknown };
        if (!dispatch?.event) return;

        switch (dispatch.event) {
          case "MESSAGE_CREATE": {
            const raw = dispatch.data as {
              id: string;
              channel_id: string;
              author_id: string;
              author_username?: string;
              author_avatar?: string | null;
              content: string;
              created_at: string;
              edited_at?: string;
              reactions?: { emoji: string; count: number; me: boolean }[];
              embeds?: object[];
              thread_id?: string;
            };
            const existing = useStore.getState().messages[raw.channel_id] ?? [];
            if (existing.some((m) => m.id === raw.id)) break;
            appendMessage(raw.channel_id, {
              id: raw.id,
              channelId: raw.channel_id,
              authorId: raw.author_id,
              authorUsername: raw.author_username ?? "Unknown",
              authorAvatar: raw.author_avatar ?? undefined,
              content: raw.content,
              createdAt: raw.created_at,
              editedAt: raw.edited_at,
              reactions: raw.reactions ?? [],
              embeds: raw.embeds ?? [],
              threadId: raw.thread_id,
            });
            const currentUserId = useStore.getState().session?.userId;
            const currentUsername = useStore.getState().session?.username ?? "";
            const isMention =
              currentUserId != null &&
              raw.author_id !== currentUserId &&
              (raw.content.includes(`@${currentUsername}`) || raw.content.includes(`@everyone`));
            if (isMention) {
              const channelName = useStore.getState().channels.find((c) => c.id === raw.channel_id)?.name;
              addInAppNotification({
                id: raw.id,
                channelId: raw.channel_id,
                channelName,
                authorId: raw.author_id,
                authorUsername: raw.author_username ?? "Unknown",
                content: raw.content,
                createdAt: raw.created_at,
              });
              if (document.hidden) {
                sendOsNotification(
                  `${raw.author_username ?? "Someone"} mentioned you`,
                  raw.content.length > 120 ? raw.content.slice(0, 120) + "…" : raw.content,
                );
              }
            }
            break;
          }

          case "MESSAGE_DELETE": {
            const raw = dispatch.data as { id: string; channel_id: string };
            onMessageDelete(raw.channel_id, raw.id);
            break;
          }

          case "MESSAGE_REACTION_ADD": {
            const raw = dispatch.data as { message_id: string; channel_id: string; user_id: string; emoji: string };
            const mine = raw.user_id === useStore.getState().session?.userId;
            updateMessageReaction(raw.channel_id, raw.message_id, raw.emoji, +1, mine);
            break;
          }

          case "MESSAGE_REACTION_REMOVE": {
            const raw = dispatch.data as { message_id: string; channel_id: string; user_id: string; emoji: string };
            const mine = raw.user_id === useStore.getState().session?.userId;
            updateMessageReaction(raw.channel_id, raw.message_id, raw.emoji, -1, mine);
            break;
          }

          case "TYPING_START": {
            const raw = dispatch.data as { channel_id: string; user_id: string; username?: string };
            setTyping(raw.channel_id, raw.username ?? raw.user_id, true);
            break;
          }

          case "VOICE_STATE_UPDATE": {
            const participants = dispatch.data as VoiceParticipant[];
            setVoiceParticipants(participants);
            break;
          }

          case "PTT_START":
            setPttActive(true);
            break;

          case "PTT_STOP":
            setPttActive(false);
            break;

          case "POLL_VOTE_ADD":
          case "POLL_VOTE_REMOVE": {
            const raw = dispatch.data as { poll_id: string; channel_id: string };
            type RawResults = { poll: Record<string, unknown>; options: { index: number; label: string; vote_count: number; voter_ids: string[] }[]; total_voters: number };
            invoke<RawResults>("get_poll_results", { channelId: raw.channel_id, pollId: raw.poll_id })
              .then((res) => {
                useStore.getState().setPollResults(raw.poll_id, {
                  poll: res.poll as any,
                  options: res.options.map((o) => ({
                    index: o.index,
                    label: o.label,
                    voteCount: o.vote_count,
                    voterIds: o.voter_ids,
                  })),
                  totalVoters: res.total_voters,
                });
              })
              .catch((e) => console.warn("[gateway] poll results refresh failed", e));
            break;
          }

          case "POLL_ENDED": {
            const raw = dispatch.data as { id: string; channel_id: string };
            useStore.getState().updatePoll(raw.channel_id, raw.id, (p) => ({ ...p, status: "ended" as const }));
            break;
          }

          case "MESSAGE_FORWARD": {
            const raw = dispatch.data as Record<string, unknown>;
            if (raw.channel_id && raw.id) {
              appendMessage(raw.channel_id as string, {
                id: raw.id as string,
                channelId: raw.channel_id as string,
                authorId: raw.author_id as string,
                authorUsername: (raw.author_username as string) ?? "",
                authorAvatar: (raw.author_avatar as string | undefined) ?? undefined,
                content: raw.content as string,
                createdAt: raw.created_at as string,
                forwardedFromMessageId: raw.forwarded_from_message_id as string | undefined,
                forwardedFromChannelId: raw.forwarded_from_channel_id as string | undefined,
                attachments: [],
                reactions: [],
              });
            }
            break;
          }

          case "GUILD_SCHEDULED_EVENT_CREATE": {
            const raw = dispatch.data as Record<string, unknown>;
            const serverId = raw.server_id as string;
            useStore.getState().upsertServerEvent(serverId, {
              id: raw.id as string,
              serverId,
              creatorId: raw.creator_id as string,
              title: raw.title as string,
              description: raw.description as string | undefined,
              startsAt: raw.starts_at as string,
              endsAt: raw.ends_at as string | undefined,
              location: raw.location as string | undefined,
              channelId: raw.channel_id as string | undefined,
              coverImage: undefined,
              status: "scheduled",
              interestedCount: 0,
              isInterested: false,
              createdAt: raw.created_at as string,
              updatedAt: raw.updated_at as string,
            });
            break;
          }

          case "GUILD_SCHEDULED_EVENT_UPDATE": {
            const raw = dispatch.data as Record<string, unknown>;
            const serverId = raw.server_id as string;
            useStore.getState().upsertServerEvent(serverId, dispatch.data as ServerEvent);
            break;
          }

          case "GUILD_SCHEDULED_EVENT_DELETE": {
            const raw = dispatch.data as { server_id: string; event_id: string };
            useStore.getState().removeServerEvent(raw.server_id, raw.event_id);
            break;
          }

          case "GUILD_SCHEDULED_EVENT_START": {
            const raw = dispatch.data as { server_id: string; event_id: string };
            const existing = useStore.getState().serverEvents[raw.server_id] ?? [];
            const ev = existing.find((e) => e.id === raw.event_id);
            if (ev) useStore.getState().upsertServerEvent(raw.server_id, { ...ev, status: "active" });
            break;
          }

          case "GUILD_SCHEDULED_EVENT_USER_ADD": {
            const raw = dispatch.data as { server_id: string; event_id: string };
            const events = useStore.getState().serverEvents[raw.server_id] ?? [];
            const ev = events.find((e) => e.id === raw.event_id);
            if (ev) useStore.getState().upsertServerEvent(raw.server_id, { ...ev, interestedCount: ev.interestedCount + 1, isInterested: true });
            break;
          }

          case "GUILD_SCHEDULED_EVENT_USER_REMOVE": {
            const raw = dispatch.data as { server_id: string; event_id: string };
            const events2 = useStore.getState().serverEvents[raw.server_id] ?? [];
            const ev2 = events2.find((e) => e.id === raw.event_id);
            if (ev2) useStore.getState().upsertServerEvent(raw.server_id, { ...ev2, interestedCount: Math.max(0, ev2.interestedCount - 1), isInterested: false });
            break;
          }

          case "GUILD_STICKERS_UPDATE": {
            const raw = dispatch.data as { guild_id: string };
            useStore.getState().loadServerStickers(raw.guild_id);
            break;
          }

          case "USER_BADGE_ADD": {
            const badge = dispatch.data as UserBadge;
            const existing = useStore.getState().userBadges[badge.userId] ?? [];
            if (!existing.find((b) => b.id === badge.id)) {
              useStore.getState().setUserBadges(badge.userId, [...existing, badge]);
            }
            break;
          }

          case "SERVER_BOOST":
          case "SERVER_TIER_UPDATE": {
            const raw = dispatch.data as { server_id?: string; serverId?: string };
            const serverId = raw.serverId ?? raw.server_id;
            if (serverId) useStore.getState().loadBoostTierInfo(serverId);
            break;
          }

          case "CANVAS_BLOCK_UPDATE": {
            const raw = dispatch.data as CanvasBlock & { reorder?: boolean };
            if (raw.reorder) {
              useStore.getState().loadCanvasBlocks(raw.channelId);
            } else {
              useStore.getState().upsertCanvasBlock(raw.channelId, raw);
            }
            break;
          }

          case "CANVAS_BLOCK_DELETE": {
            const raw = dispatch.data as { channel_id: string; block_id: string };
            useStore.getState().removeCanvasBlock(raw.channel_id, raw.block_id);
            break;
          }

          case "RELATIONSHIP_UPDATE": {
            useStore.getState().loadRelationships();
            const raw = dispatch.data as {
              id: string;
              type: "pending_incoming" | "friend" | "removed";
              user: { id: string; username: string; display_name?: string; avatar_url?: string };
            };
            if (raw.type === "pending_incoming") {
              const name = raw.user.display_name ?? raw.user.username;
              addInAppNotification({
                id: raw.id,
                channelId: "",
                channelName: undefined,
                authorId: raw.user.id,
                authorUsername: raw.user.username,
                content: `${name} sent you a friend request`,
                createdAt: new Date().toISOString(),
              });
              if (document.hidden) {
                sendOsNotification("Friend Request", `${name} sent you a friend request`);
              }
            } else if (raw.type === "friend") {
              const name = raw.user.display_name ?? raw.user.username;
              addInAppNotification({
                id: raw.id,
                channelId: "",
                channelName: undefined,
                authorId: raw.user.id,
                authorUsername: raw.user.username,
                content: `${name} accepted your friend request`,
                createdAt: new Date().toISOString(),
              });
            }
            break;
          }
        }
      };

      ws.onclose = () => {
        if (heartbeatTimer.current) clearInterval(heartbeatTimer.current);
        if (heartbeatTimeout.current) clearTimeout(heartbeatTimeout.current);
        if (destroyed || closingIntentional.current) {
          setGatewayStatus("offline");
          return;
        }
        setGatewayStatus("offline");
        reconnectAttempts.current += 1;
        const delay = Math.min(30_000, 1_000 * 2 ** Math.min(reconnectAttempts.current, 5));
        reconnectTimer.current = setTimeout(connect, delay);
      };

      ws.onerror = (err) => {
        console.error("[gateway] error", err);
        ws.close();
      };
    };

    connect();

    return () => {
      destroyed = true;
      closingIntentional.current = true;
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      if (heartbeatTimer.current) clearInterval(heartbeatTimer.current);
      if (heartbeatTimeout.current) clearTimeout(heartbeatTimeout.current);
      wsRef.current?.close();
    };
  }, [session?.accessToken, session?.serverUrl, appendMessage, setVoiceParticipants, setPttActive, setTyping, updateMessageReaction, addInAppNotification, setGatewayStatus, onMessageDelete]);
}

// ── useVoice — Voice channel WebRTC signaling hook ───────────────────────────
//
// Handles the signaling layer for voice channels:
//   1. POST /voice/channels/:id/join → get voice WS URL + session ID
//   2. Connect signaling WebSocket (str0m protocol)
//   3. Identify → SDP offer/answer → ICE exchange
//   4. Audio MediaStream via getUserMedia
//   5. Expose controls: mute, deafen, leave
//
// The actual audio rendering is left to the browser's WebRTC stack once
// the RTCPeerConnection is established via the SFU answer.

import { useCallback, useEffect, useRef, useState } from "react";
import { useStore } from "../store";

export type VoiceStatus = "idle" | "connecting" | "connected" | "error";

export interface VoiceParticipant {
  user_id: string;
  username: string;
  muted: boolean;
  deafened: boolean;
}

export interface UseVoiceReturn {
  status: VoiceStatus;
  channelId: string | null;
  participants: VoiceParticipant[];
  muted: boolean;
  deafened: boolean;
  error: string | null;
  joinVoice: (channelId: string) => Promise<void>;
  leaveVoice: () => void;
  toggleMute: () => void;
  toggleDeafen: () => void;
}

export function useVoice(): UseVoiceReturn {
  const { session } = useStore();

  const [status, setStatus]           = useState<VoiceStatus>("idle");
  const [channelId, setChannelId]     = useState<string | null>(null);
  const [participants, setParticipants] = useState<VoiceParticipant[]>([]);
  const [muted, setMuted]             = useState(false);
  const [deafened, setDeafened]       = useState(false);
  const [error, setError]             = useState<string | null>(null);

  const wsRef        = useRef<WebSocket | null>(null);
  const pcRef        = useRef<RTCPeerConnection | null>(null);
  const streamRef    = useRef<MediaStream | null>(null);
  const heartbeatRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Clean up all voice resources
  const cleanup = useCallback(() => {
    if (heartbeatRef.current) clearInterval(heartbeatRef.current);
    wsRef.current?.close();
    pcRef.current?.close();
    streamRef.current?.getTracks().forEach((t) => t.stop());
    wsRef.current  = null;
    pcRef.current  = null;
    streamRef.current = null;
    setStatus("idle");
    setChannelId(null);
    setParticipants([]);
  }, []);

  // Cleanup on unmount
  useEffect(() => () => cleanup(), [cleanup]);

  const joinVoice = useCallback(async (targetChannelId: string) => {
    if (!session) return;
    if (wsRef.current) cleanup(); // leave current channel first

    setStatus("connecting");
    setError(null);
    setChannelId(targetChannelId);

    try {
      // 1. Pre-flight: get voice WS URL and session token
      const preflight = await fetch(
        `${session.serverUrl}/api/v1/voice/channels/${targetChannelId}/join`,
        { method: "POST", headers: { Authorization: `Bearer ${session.accessToken}` } }
      );
      if (!preflight.ok) {
        const msg = await preflight.text().catch(() => `HTTP ${preflight.status}`);
        throw new Error(msg);
      }
      const { voice_ws_url, session_id, participants: initial } = await preflight.json();

      // Update participant list from preflight
      setParticipants(initial ?? []);

      // 2. Open signaling WebSocket
      const ws = new WebSocket(voice_ws_url);
      wsRef.current = ws;

      ws.onclose = () => {
        if (status !== "idle") {
          setStatus("idle");
          setChannelId(null);
        }
      };

      ws.onerror = () => {
        setError("Voice connection lost");
        setStatus("error");
      };

      ws.onmessage = async (evt) => {
        try {
          const msg = JSON.parse(evt.data as string) as Record<string, unknown>;
          await handleSignal(msg, ws, targetChannelId, session_id);
        } catch {
          // ignore malformed frames
        }
      };

      // 3. Request microphone access
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
        streamRef.current = stream;
      } catch {
        // No mic — join listen-only
        streamRef.current = null;
      }

    } catch (e) {
      setError((e as Error).message);
      setStatus("error");
      cleanup();
    }
  }, [session, cleanup, status]);

  const handleSignal = async (
    msg: Record<string, unknown>,
    ws: WebSocket,
    targetChannelId: string,
    sessionId: string,
  ) => {
    const op = msg.op as string;

    if (op === "Hello") {
      // Send Identify
      // No token: the voice server authenticated this socket from the
      // proxy-injected identity header at the upgrade, before the connection
      // existed. The channel and session ids are routing, not credentials.
      ws.send(JSON.stringify({
        op: "Identify",
        d: {
          channel_id: targetChannelId,
          session_id: sessionId,
        },
      }));

      // Start heartbeat
      const interval = (msg.d as Record<string, number>)?.heartbeat_interval ?? 30_000;
      heartbeatRef.current = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ op: "Heartbeat" }));
        }
      }, interval);
    }

    if (op === "Ready") {
      // Create RTCPeerConnection and generate SDP offer
      const pc = new RTCPeerConnection({
        iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
      });
      pcRef.current = pc;

      // Add local audio tracks
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((t) => pc.addTrack(t, streamRef.current!));
      }

      // Play remote audio
      pc.ontrack = (evt) => {
        const audio = new Audio();
        audio.srcObject = evt.streams[0];
        audio.autoplay = true;
        if (deafened) audio.volume = 0;
      };

      // Send ICE candidates to server via signaling WS
      pc.onicecandidate = (evt) => {
        if (evt.candidate && ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({
            op: "IceCandidate",
            d: { candidate: evt.candidate.candidate },
          }));
        }
      };

      // Generate offer
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);

      ws.send(JSON.stringify({
        op: "Offer",
        d: { sdp: offer.sdp },
      }));

      setStatus("connected");
    }

    if (op === "Answer") {
      // Apply SFU's SDP answer
      const d = msg.d as { sdp: string };
      await pcRef.current?.setRemoteDescription({ type: "answer", sdp: d.sdp });
    }

    if (op === "ServerIceCandidate") {
      const d = msg.d as { candidate: string };
      await pcRef.current?.addIceCandidate({ candidate: d.candidate, sdpMid: "0", sdpMLineIndex: 0 });
    }

    if (op === "ParticipantUpdate") {
      const d = msg.d as { participants: VoiceParticipant[] };
      setParticipants(d.participants ?? []);
    }
  };

  const leaveVoice = useCallback(() => {
    // Best-effort REST leave so server cleans up immediately
    if (session && channelId) {
      fetch(`${session.serverUrl}/api/v1/voice/channels/${channelId}/leave`, {
        method: "POST",
        headers: { Authorization: `Bearer ${session.accessToken}` },
      }).catch(() => {});
    }
    cleanup();
  }, [session, channelId, cleanup]);

  const toggleMute = useCallback(() => {
    const newMuted = !muted;
    setMuted(newMuted);
    streamRef.current?.getAudioTracks().forEach((t) => { t.enabled = !newMuted; });
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ op: "UpdateVoiceState", d: { muted: newMuted } }));
    }
  }, [muted]);

  const toggleDeafen = useCallback(() => {
    const newDeafened = !deafened;
    setDeafened(newDeafened);
    // Deafen: mute mic too (standard behaviour)
    if (newDeafened && !muted) toggleMute();
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ op: "UpdateVoiceState", d: { deafened: newDeafened } }));
    }
  }, [deafened, muted, toggleMute]);

  return {
    status, channelId, participants, muted, deafened, error,
    joinVoice, leaveVoice, toggleMute, toggleDeafen,
  };
}

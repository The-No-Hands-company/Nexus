/**
 * VoiceCollabPanel — Phase 22 voice & real-time collaboration features.
 *
 * Tabs:
 *   Layout      — Video layout configuration
 *   Backgrounds — Virtual background manager
 *   Streams     — Live stream viewer
 *   Breakout    — Breakout room management
 *   Collab      — Collaborative sessions
 *   Spatial     — Spatial audio config
 *   Presets     — Voice preset browser
 */

import { useEffect, useState, useCallback } from "react";
import { invoke } from "../invoke";
import clsx from "clsx";

type Tab = "layout" | "backgrounds" | "streams" | "breakout" | "collab" | "spatial" | "presets";

interface Props {
  channelId?: string;
  serverId?: string;
}

export default function VoiceCollabPanel({ channelId, serverId }: Props) {
  const [tab, setTab] = useState<Tab>("layout");

  const tabs: { key: Tab; label: string }[] = [
    { key: "layout", label: "Layout" },
    { key: "backgrounds", label: "Backgrounds" },
    { key: "streams", label: "Streams" },
    { key: "breakout", label: "Breakout" },
    { key: "collab", label: "Collab" },
    { key: "spatial", label: "Spatial" },
    { key: "presets", label: "Presets" },
  ];

  return (
    <div className="flex flex-col h-full">
      <div className="flex gap-1 p-2 border-b border-gray-700 flex-wrap">
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={clsx(
              "px-3 py-1.5 rounded text-sm",
              tab === t.key
                ? "bg-indigo-600 text-white"
                : "text-gray-400 hover:text-white hover:bg-gray-700"
            )}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div className="flex-1 overflow-y-auto p-4">
        {tab === "layout" && <LayoutTab channelId={channelId} />}
        {tab === "backgrounds" && <BackgroundsTab />}
        {tab === "streams" && <StreamsTab channelId={channelId} serverId={serverId} />}
        {tab === "breakout" && <BreakoutTab channelId={channelId} />}
        {tab === "collab" && <CollabTab channelId={channelId} />}
        {tab === "spatial" && <SpatialTab channelId={channelId} />}
        {tab === "presets" && <PresetsTab />}
      </div>
    </div>
  );
}

function LayoutTab({ channelId }: { channelId?: string }) {
  const [layout, setLayout] = useState<any>(null);
  useEffect(() => {
    if (!channelId) return;
    invoke("get_video_layout", { channelId }).then(setLayout).catch(console.error);
  }, [channelId]);
  if (!channelId) return <p className="text-gray-400 text-sm">Join a voice channel to configure video layout.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Video Layout</h3>
      {layout ? (
        <div className="bg-gray-800 rounded p-4 text-sm text-gray-300">
          <div>Type: <span className="text-white">{layout.layout_type}</span></div>
          <div className="mt-1">PiP: {layout.pip_enabled ? "Yes" : "No"}</div>
        </div>
      ) : (
        <p className="text-gray-400 text-sm">No custom layout set. Using default grid view.</p>
      )}
    </div>
  );
}

function BackgroundsTab() {
  const [bgs, setBgs] = useState<any[]>([]);
  const reload = useCallback(() => {
    invoke("list_virtual_backgrounds").then(setBgs).catch(console.error);
  }, []);
  useEffect(reload, [reload]);
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Virtual Backgrounds</h3>
      {bgs.length === 0 ? (
        <p className="text-gray-400 text-sm">No custom backgrounds. Add one to personalize your video.</p>
      ) : (
        <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
          {bgs.map((b: any) => (
            <div key={b.id} className="bg-gray-800 rounded p-3 text-sm text-center">
              <div className="text-white font-medium">{b.name}</div>
              <div className="text-gray-500 text-xs">{b.bg_type}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function StreamsTab({ channelId, serverId }: { channelId?: string; serverId?: string }) {
  const [streams, setStreams] = useState<any[]>([]);
  useEffect(() => {
    if (!channelId) return;
    invoke("list_live_streams", { serverId, channelId }).then(setStreams).catch(console.error);
  }, [channelId, serverId]);
  if (!channelId) return <p className="text-gray-400 text-sm">Select a channel to view live streams.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Live Streams</h3>
      {streams.length === 0 ? (
        <p className="text-gray-400 text-sm">No active streams in this channel.</p>
      ) : (
        <div className="space-y-2">
          {streams.map((s: any) => (
            <div key={s.id} className="bg-gray-800 rounded p-3 text-sm flex justify-between">
              <span className="text-white">{s.title || "Untitled Stream"}</span>
              <span className={clsx("text-xs px-2 py-0.5 rounded", s.is_active ? "bg-red-900 text-red-300" : "bg-gray-700 text-gray-400")}>
                {s.is_active ? "LIVE" : "Ended"}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function BreakoutTab({ channelId }: { channelId?: string }) {
  const [rooms, setRooms] = useState<any[]>([]);
  useEffect(() => {
    if (!channelId) return;
    invoke("list_breakout_rooms", { channelId }).then(setRooms).catch(console.error);
  }, [channelId]);
  if (!channelId) return <p className="text-gray-400 text-sm">Join a voice channel to manage breakout rooms.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Breakout Rooms</h3>
      {rooms.length === 0 ? (
        <p className="text-gray-400 text-sm">No breakout rooms. Create one to split participants.</p>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
          {rooms.map((r: any) => (
            <div key={r.id} className="bg-gray-800 rounded p-3 text-sm">
              <div className="text-white font-medium">{r.name}</div>
              <div className="text-gray-400">Capacity: {r.capacity}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function CollabTab({ channelId }: { channelId?: string }) {
  const [sessions, setSessions] = useState<any[]>([]);
  useEffect(() => {
    if (!channelId) return;
    invoke("list_collab_sessions", { channelId }).then(setSessions).catch(console.error);
  }, [channelId]);
  if (!channelId) return <p className="text-gray-400 text-sm">Select a channel for collaborative sessions.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Collaborative Sessions</h3>
      {sessions.length === 0 ? (
        <p className="text-gray-400 text-sm">No active sessions. Start a whiteboard or code collaboration.</p>
      ) : (
        <div className="space-y-2">
          {sessions.map((s: any) => (
            <div key={s.id} className="bg-gray-800 rounded p-3 text-sm flex justify-between">
              <span className="text-white">{s.session_type}</span>
              <span className={clsx("text-xs px-2 py-0.5 rounded", s.is_active ? "bg-green-900 text-green-300" : "bg-gray-700 text-gray-400")}>
                {s.is_active ? "Active" : "Ended"}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SpatialTab({ channelId }: { channelId?: string }) {
  const [config, setConfig] = useState<any>(null);
  useEffect(() => {
    if (!channelId) return;
    invoke("get_spatial_audio_config", { channelId }).then(setConfig).catch(console.error);
  }, [channelId]);
  if (!channelId) return <p className="text-gray-400 text-sm">Join a voice channel to configure spatial audio.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Spatial Audio</h3>
      {config ? (
        <div className="bg-gray-800 rounded p-4 text-sm text-gray-300 space-y-1">
          <div>Preset: <span className="text-white">{config.preset}</span></div>
          <div>Room: {config.room_width}×{config.room_depth}×{config.room_height}</div>
          <div>HRTF: {config.hrtf_enabled ? "Yes" : "No"} · Ambisonics: {config.ambisonics_order}</div>
        </div>
      ) : (
        <p className="text-gray-400 text-sm">No spatial audio config. Using default settings.</p>
      )}
    </div>
  );
}

function PresetsTab() {
  const [presets, setPresets] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_voice_presets").then(setPresets).catch(console.error);
  }, []);
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Voice Presets</h3>
      {presets.length === 0 ? (
        <p className="text-gray-400 text-sm">No voice presets available.</p>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
          {presets.map((p: any) => (
            <div key={p.name} className="bg-gray-800 rounded p-3 text-sm">
              <div className="text-white font-medium">{p.name}</div>
              <div className="text-gray-400 text-xs mt-1">
                Noise gate: {p.noise_gate_db}dB · Comp: {p.compressor_ratio}:1 · EQ: {p.eq_preset}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

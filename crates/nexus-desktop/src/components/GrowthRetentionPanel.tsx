/**
 * GrowthRetentionPanel — Phase 23 growth & retention features.
 *
 * Tabs:
 *   Discover   — Server recommendations
 *   Gamify     — Gamification config, XP, achievements
 *   Onboard    — Onboarding flow editor
 *   Devices    — Cross-device session management
 *   Sync       — Sync cursors & offline queue
 */

import { useEffect, useState, useCallback } from "react";
import { invoke } from "../invoke";
import clsx from "clsx";

type Tab = "discover" | "gamify" | "onboard" | "devices" | "sync";

interface Props {
  serverId?: string;
}

export default function GrowthRetentionPanel({ serverId }: Props) {
  const [tab, setTab] = useState<Tab>("discover");

  const tabs: { key: Tab; label: string }[] = [
    { key: "discover", label: "Discover" },
    { key: "gamify", label: "Gamification" },
    { key: "onboard", label: "Onboarding" },
    { key: "devices", label: "Devices" },
    { key: "sync", label: "Sync" },
  ];

  return (
    <div className="flex flex-col h-full">
      <div className="flex gap-1 p-2 border-b border-gray-700">
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
        {tab === "discover" && <DiscoverTab />}
        {tab === "gamify" && <GamifyTab serverId={serverId} />}
        {tab === "onboard" && <OnboardTab serverId={serverId} />}
        {tab === "devices" && <DevicesTab />}
        {tab === "sync" && <SyncTab />}
      </div>
    </div>
  );
}

function DiscoverTab() {
  const [recs, setRecs] = useState<any[]>([]);
  const reload = useCallback(() => {
    invoke("list_recommendations").then(setRecs).catch(console.error);
  }, []);
  useEffect(reload, [reload]);

  const dismiss = async (recId: string) => {
    await invoke("dismiss_recommendation", { recId });
    reload();
  };

  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Recommended Servers</h3>
      {recs.length === 0 ? (
        <p className="text-gray-400 text-sm">No recommendations right now. Join more servers to get personalized picks.</p>
      ) : (
        <div className="space-y-2">
          {recs.map((r: any) => (
            <div key={r.id} className="bg-gray-800 rounded p-3 text-sm flex justify-between items-center">
              <div>
                <div className="text-white font-medium">{r.server_id}</div>
                <div className="text-gray-400 text-xs">{r.reason} · Score: {(r.score * 100).toFixed(0)}%</div>
              </div>
              <button onClick={() => dismiss(r.server_id)} className="text-gray-500 hover:text-red-400 text-xs">Dismiss</button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function GamifyTab({ serverId }: { serverId?: string }) {
  const [config, setConfig] = useState<any>(null);
  const [leaderboard, setLeaderboard] = useState<any[]>([]);
  const [achievements, setAchievements] = useState<any[]>([]);
  useEffect(() => {
    if (!serverId) return;
    invoke("get_gamification_config", { serverId }).then(setConfig).catch(console.error);
    invoke("list_xp_leaderboard", { serverId }).then(setLeaderboard).catch(console.error);
    invoke("list_achievements", { serverId }).then(setAchievements).catch(console.error);
  }, [serverId]);
  if (!serverId) return <p className="text-gray-400 text-sm">Select a server for gamification settings.</p>;
  return (
    <div className="space-y-4">
      {config && (
        <div className="bg-gray-800 rounded p-4 text-sm">
          <h4 className="text-white font-medium mb-2">Gamification Config</h4>
          <div className="grid grid-cols-2 gap-2 text-gray-300">
            <div>Enabled: {config.enabled ? "Yes" : "No"}</div>
            <div>XP/message: {config.xp_per_message}</div>
            <div>XP/reaction: {config.xp_per_reaction}</div>
            <div>XP/voice min: {config.xp_per_voice_min}</div>
            <div>Streaks: {config.streak_enabled ? "Yes" : "No"}</div>
          </div>
        </div>
      )}
      <div>
        <h4 className="text-white font-medium mb-2">Leaderboard</h4>
        {leaderboard.length === 0 ? (
          <p className="text-gray-400 text-sm">No XP data yet.</p>
        ) : (
          <div className="space-y-1">
            {leaderboard.slice(0, 10).map((u: any, i: number) => (
              <div key={u.user_id} className="bg-gray-800 rounded p-2 text-sm flex justify-between">
                <span className="text-gray-300">#{i + 1} {u.user_id}</span>
                <span className="text-indigo-400 font-medium">{u.xp} XP (Lv {u.level})</span>
              </div>
            ))}
          </div>
        )}
      </div>
      <div>
        <h4 className="text-white font-medium mb-2">Achievements</h4>
        {achievements.length === 0 ? (
          <p className="text-gray-400 text-sm">No achievements defined yet.</p>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            {achievements.map((a: any) => (
              <div key={a.id} className="bg-gray-800 rounded p-3 text-sm">
                <div className="flex items-center gap-2">
                  {a.icon_url && <img src={a.icon_url} className="w-6 h-6 rounded" alt="" />}
                  <span className="text-white font-medium">{a.name}</span>
                </div>
                {a.description && <div className="text-gray-400 text-xs mt-1">{a.description}</div>}
                <div className="text-indigo-400 text-xs mt-1">+{a.reward_xp} XP</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function OnboardTab({ serverId }: { serverId?: string }) {
  const [flow, setFlow] = useState<any>(null);
  useEffect(() => {
    if (!serverId) return;
    invoke("get_onboarding_flow", { serverId }).then(setFlow).catch(console.error);
  }, [serverId]);
  if (!serverId) return <p className="text-gray-400 text-sm">Select a server to manage onboarding flow.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Onboarding Flow</h3>
      {flow ? (
        <div className="bg-gray-800 rounded p-4 text-sm text-gray-300">
          <div>Adaptive: {flow.adaptive ? "Yes" : "No"} · Skip completed: {flow.skip_completed ? "Yes" : "No"}</div>
          <div className="mt-2 text-gray-400 text-xs">Steps: {JSON.stringify(flow.steps).substring(0, 200)}...</div>
        </div>
      ) : (
        <p className="text-gray-400 text-sm">No onboarding flow configured for this server.</p>
      )}
    </div>
  );
}

function DevicesTab() {
  const [sessions, setSessions] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_device_sessions").then(setSessions).catch(console.error);
  }, []);
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Device Sessions</h3>
      {sessions.length === 0 ? (
        <p className="text-gray-400 text-sm">No active device sessions.</p>
      ) : (
        <div className="space-y-2">
          {sessions.map((s: any) => (
            <div key={s.id} className="bg-gray-800 rounded p-3 text-sm flex justify-between">
              <div>
                <div className="text-white font-medium">{s.device_type}</div>
                <div className="text-gray-400 text-xs">ID: {s.device_id}</div>
              </div>
              <span className={clsx("text-xs px-2 py-0.5 rounded self-center", s.is_active ? "bg-green-900 text-green-300" : "bg-gray-700 text-gray-400")}>
                {s.is_active ? "Active" : "Inactive"}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SyncTab() {
  const [offline, setOffline] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_pending_offline").then(setOffline).catch(console.error);
  }, []);
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Offline Queue</h3>
      {offline.length === 0 ? (
        <p className="text-gray-400 text-sm">All caught up — no pending offline actions.</p>
      ) : (
        <div className="space-y-2">
          {offline.map((o: any) => (
            <div key={o.id} className="bg-gray-800 rounded p-3 text-sm text-gray-300">
              {o.action_type} · Status: {o.status}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

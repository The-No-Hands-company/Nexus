/**
 * AiIntelligencePanel — Phase 21 AI features panel.
 *
 * Tabs:
 *   Suggestions — AI suggestion feed per channel
 *   Summaries   — Thread auto-summaries
 *   Moderation  — Toxicity flags & raid detections
 *   Transcripts — Voice-to-text transcripts
 *   Consent     — Per-user AI feature opt-in/out
 */

import { useEffect, useState } from "react";
import { invoke } from "../invoke";
import clsx from "clsx";

type Tab = "suggestions" | "summaries" | "moderation" | "transcripts" | "consent";

interface Props {
  serverId?: string;
  channelId?: string;
}

export default function AiIntelligencePanel({ serverId, channelId }: Props) {
  const [tab, setTab] = useState<Tab>("suggestions");

  const tabs: { key: Tab; label: string }[] = [
    { key: "suggestions", label: "Suggestions" },
    { key: "summaries", label: "Summaries" },
    { key: "moderation", label: "Moderation" },
    { key: "transcripts", label: "Transcripts" },
    { key: "consent", label: "Consent" },
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
        {tab === "suggestions" && <SuggestionsTab channelId={channelId} />}
        {tab === "summaries" && <SummariesTab channelId={channelId} />}
        {tab === "moderation" && <ModerationTab serverId={serverId} />}
        {tab === "transcripts" && <TranscriptsTab channelId={channelId} />}
        {tab === "consent" && <ConsentTab serverId={serverId} />}
      </div>
    </div>
  );
}

function SuggestionsTab({ channelId }: { channelId?: string }) {
  const [items, setItems] = useState<any[]>([]);
  useEffect(() => {
    if (!channelId) return;
    invoke("list_ai_suggestions", { channelId }).then(setItems).catch(console.error);
  }, [channelId]);
  if (!channelId) return <p className="text-gray-400 text-sm">Select a channel to view AI suggestions.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">AI Suggestions</h3>
      {items.length === 0 ? (
        <p className="text-gray-400 text-sm">No suggestions yet.</p>
      ) : (
        <div className="space-y-2">
          {items.map((s: any) => (
            <div key={s.id} className="bg-gray-800 rounded p-3 text-sm">
              <div className="text-indigo-400 font-medium">{s.suggestion_type}</div>
              <div className="text-gray-300 mt-1">{s.content}</div>
              <div className="text-gray-500 text-xs mt-1">Model: {s.model_name}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SummariesTab({ channelId }: { channelId?: string }) {
  const [summary, setSummary] = useState<any>(null);
  useEffect(() => {
    if (!channelId) return;
    invoke("get_thread_summary", { channelId }).then(setSummary).catch(console.error);
  }, [channelId]);
  if (!channelId) return <p className="text-gray-400 text-sm">Select a channel to view summaries.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Thread Summary</h3>
      {summary ? (
        <div className="bg-gray-800 rounded p-4 text-sm text-gray-300">
          <p>{summary.summary}</p>
          <div className="text-gray-500 text-xs mt-2">Messages: {summary.message_count} · Model: {summary.model_name}</div>
        </div>
      ) : (
        <p className="text-gray-400 text-sm">No summary available.</p>
      )}
    </div>
  );
}

function ModerationTab({ serverId }: { serverId?: string }) {
  const [toxicity, setToxicity] = useState<any[]>([]);
  const [raids, setRaids] = useState<any[]>([]);
  useEffect(() => {
    if (!serverId) return;
    invoke("list_flagged_toxicity", { serverId }).then(setToxicity).catch(console.error);
    invoke("list_raid_detections", { serverId }).then(setRaids).catch(console.error);
  }, [serverId]);
  if (!serverId) return <p className="text-gray-400 text-sm">Select a server to view moderation.</p>;
  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-white">Toxicity Flags</h3>
        {toxicity.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No flagged messages.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {toxicity.map((t: any) => (
              <div key={t.id} className="bg-gray-800 rounded p-3 text-sm">
                <div className="flex justify-between">
                  <span className="text-red-400">Score: {(t.score * 100).toFixed(0)}%</span>
                  <span className="text-gray-500 text-xs">{t.model_name}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      <div>
        <h3 className="text-lg font-semibold text-white">Raid Detections</h3>
        {raids.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No raids detected.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {raids.map((r: any) => (
              <div key={r.id} className="bg-gray-800 rounded p-3 text-sm">
                <span className="text-orange-400 font-medium">{r.severity}</span>
                <span className="text-gray-400 ml-2">{r.detection_type}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function TranscriptsTab({ channelId }: { channelId?: string }) {
  const [items, setItems] = useState<any[]>([]);
  useEffect(() => {
    if (!channelId) return;
    invoke("list_voice_transcripts", { channelId }).then(setItems).catch(console.error);
  }, [channelId]);
  if (!channelId) return <p className="text-gray-400 text-sm">Select a voice channel for transcripts.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Voice Transcripts</h3>
      {items.length === 0 ? (
        <p className="text-gray-400 text-sm">No transcripts yet.</p>
      ) : (
        <div className="space-y-2">
          {items.map((t: any) => (
            <div key={t.id} className="bg-gray-800 rounded p-3 text-sm">
              <span className="text-gray-300">{t.text}</span>
              <div className="text-gray-500 text-xs mt-1">
                {t.language} · Confidence: {(t.confidence * 100).toFixed(0)}%
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ConsentTab({ serverId }: { serverId?: string }) {
  const [consents, setConsents] = useState<any[]>([]);
  useEffect(() => {
    if (!serverId) return;
    invoke("list_ai_consent", { serverId }).then(setConsents).catch(console.error);
  }, [serverId]);
  if (!serverId) return <p className="text-gray-400 text-sm">Select a server to manage AI consent.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">AI Consent Settings</h3>
      {consents.length === 0 ? (
        <p className="text-gray-400 text-sm">No AI consent settings configured.</p>
      ) : (
        <div className="space-y-2">
          {consents.map((c: any, i: number) => (
            <div key={i} className="bg-gray-800 rounded p-3 text-sm flex justify-between items-center">
              <span className="text-gray-300">{c.feature}</span>
              <span className={clsx("text-xs px-2 py-0.5 rounded", c.enabled ? "bg-green-900 text-green-300" : "bg-red-900 text-red-300")}>
                {c.enabled ? "Enabled" : "Disabled"}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

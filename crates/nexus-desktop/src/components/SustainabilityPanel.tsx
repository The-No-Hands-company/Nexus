/**
 * SustainabilityPanel — Phase 24 sustainability & extensibility features.
 *
 * Tabs:
 *   Protocol    — Protocol version management
 *   Governance  — Polls & proposals
 *   Badges      — Contributor badges
 *   Security    — Security audits & vulnerability records
 *   Tutorials   — Tutorial progress & migration guides
 */

import { useEffect, useState, useCallback } from "react";
import { invoke } from "../invoke";
import clsx from "clsx";

type Tab = "protocol" | "governance" | "badges" | "security" | "tutorials";

interface Props {
  serverId?: string;
  userId?: string;
}

export default function SustainabilityPanel({ serverId, userId }: Props) {
  const [tab, setTab] = useState<Tab>("governance");

  const tabs: { key: Tab; label: string }[] = [
    { key: "protocol", label: "Protocol" },
    { key: "governance", label: "Governance" },
    { key: "badges", label: "Badges" },
    { key: "security", label: "Security" },
    { key: "tutorials", label: "Tutorials" },
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
        {tab === "protocol" && <ProtocolTab />}
        {tab === "governance" && <GovernanceTab serverId={serverId} />}
        {tab === "badges" && <BadgesTab userId={userId} />}
        {tab === "security" && <SecurityTab />}
        {tab === "tutorials" && <TutorialsTab />}
      </div>
    </div>
  );
}

function ProtocolTab() {
  const [versions, setVersions] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_protocol_versions", { protocol: "nexus" }).then(setVersions).catch(console.error);
  }, []);
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Protocol Versions</h3>
      {versions.length === 0 ? (
        <p className="text-gray-400 text-sm">No protocol versions registered.</p>
      ) : (
        <div className="space-y-2">
          {versions.map((v: any) => (
            <div key={v.id} className="bg-gray-800 rounded p-3 text-sm flex justify-between">
              <div>
                <span className="text-white font-medium">{v.protocol} v{v.version}</span>
              </div>
              <span className={clsx(
                "text-xs px-2 py-0.5 rounded",
                v.status === "stable" ? "bg-green-900 text-green-300" :
                v.status === "deprecated" ? "bg-red-900 text-red-300" :
                "bg-yellow-900 text-yellow-300"
              )}>
                {v.status}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function GovernanceTab({ serverId }: { serverId?: string }) {
  const [polls, setPolls] = useState<any[]>([]);
  const [proposals, setProposals] = useState<any[]>([]);
  useEffect(() => {
    if (!serverId) return;
    invoke("list_governance_polls", { serverId }).then(setPolls).catch(console.error);
    invoke("list_governance_proposals", { serverId }).then(setProposals).catch(console.error);
  }, [serverId]);
  if (!serverId) return <p className="text-gray-400 text-sm">Select a server to view governance.</p>;
  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-white">Polls</h3>
        {polls.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No active polls.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {polls.map((p: any) => (
              <div key={p.id} className="bg-gray-800 rounded p-3 text-sm">
                <div className="text-white font-medium">{p.title}</div>
                {p.description && <div className="text-gray-400 text-xs mt-1">{p.description}</div>}
                <div className="flex gap-3 mt-2 text-gray-500 text-xs">
                  <span>Type: {p.poll_type}</span>
                  <span>Status: {p.status}</span>
                  <span>Participation: {(p.min_participation * 100).toFixed(0)}% min</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      <div>
        <h3 className="text-lg font-semibold text-white">Proposals</h3>
        {proposals.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No proposals yet.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {proposals.map((p: any) => (
              <div key={p.id} className="bg-gray-800 rounded p-3 text-sm">
                <div className="text-white font-medium">{p.title}</div>
                <div className="text-gray-400 text-xs mt-1 line-clamp-2">{p.body}</div>
                <div className="text-gray-500 text-xs mt-1">Status: {p.status}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function BadgesTab({ userId }: { userId?: string }) {
  const [badges, setBadges] = useState<any[]>([]);
  useEffect(() => {
    if (!userId) return;
    invoke("list_contributor_badges", { userId }).then(setBadges).catch(console.error);
  }, [userId]);
  if (!userId) return <p className="text-gray-400 text-sm">User context required to view badges.</p>;
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Contributor Badges</h3>
      {badges.length === 0 ? (
        <p className="text-gray-400 text-sm">No badges yet. Contribute to earn recognition!</p>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
          {badges.map((b: any) => (
            <div key={b.id} className="bg-gray-800 rounded p-3 text-sm">
              <div className="text-indigo-400 font-medium">{b.badge_type}</div>
              {b.source && <div className="text-gray-400 text-xs">{b.source}</div>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SecurityTab() {
  const [audits, setAudits] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_security_audits").then(setAudits).catch(console.error);
  }, []);
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Security Audits</h3>
      {audits.length === 0 ? (
        <p className="text-gray-400 text-sm">No security audits recorded.</p>
      ) : (
        <div className="space-y-2">
          {audits.map((a: any) => (
            <div key={a.id} className="bg-gray-800 rounded p-3 text-sm flex justify-between">
              <div>
                <span className="text-white">{a.audit_type}</span>
                {a.auditor && <span className="text-gray-400 ml-2">by {a.auditor}</span>}
              </div>
              <span className={clsx(
                "text-xs px-2 py-0.5 rounded",
                a.status === "passed" ? "bg-green-900 text-green-300" :
                a.status === "failed" ? "bg-red-900 text-red-300" :
                "bg-yellow-900 text-yellow-300"
              )}>
                {a.status}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function TutorialsTab() {
  const [progress, setProgress] = useState<any[]>([]);
  const [guides, setGuides] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_tutorial_progress").then(setProgress).catch(console.error);
    invoke("list_migration_guides").then(setGuides).catch(console.error);
  }, []);
  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-white">Your Tutorial Progress</h3>
        {progress.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No tutorials started.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {progress.map((p: any) => (
              <div key={p.tutorial_id} className="bg-gray-800 rounded p-3 text-sm flex justify-between">
                <span className="text-white">{p.tutorial_id}</span>
                <span className={clsx("text-xs px-2 py-0.5 rounded", p.completed ? "bg-green-900 text-green-300" : "bg-yellow-900 text-yellow-300")}>
                  {p.completed ? "Complete" : "In Progress"}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
      <div>
        <h3 className="text-lg font-semibold text-white">Migration Guides</h3>
        {guides.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No migration guides available.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {guides.map((g: any) => (
              <div key={g.id} className="bg-gray-800 rounded p-3 text-sm">
                <div className="text-white font-medium">{g.title}</div>
                <div className="text-gray-400 text-xs">From: {g.from_platform} · Version: {g.version}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

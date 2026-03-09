/**
 * ScalabilityPanel — Admin panel for Phase 20 scalability features.
 *
 * Tabs:
 *   Scaling   — scaling config management
 *   SFU Nodes — voice infrastructure nodes
 *   Federation — batch & route overview
 *   Quality   — voice quality logs
 *   Metrics   — operational metrics & upgrade records
 */

import { useEffect, useState } from "react";
import { invoke } from "../invoke";
import clsx from "clsx";

type Tab = "scaling" | "sfu" | "federation" | "quality" | "metrics";

export default function ScalabilityPanel() {
  const [tab, setTab] = useState<Tab>("scaling");

  const tabs: { key: Tab; label: string }[] = [
    { key: "scaling", label: "Scaling" },
    { key: "sfu", label: "SFU Nodes" },
    { key: "federation", label: "Federation" },
    { key: "quality", label: "Quality" },
    { key: "metrics", label: "Metrics" },
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
        {tab === "scaling" && <ScalingTab />}
        {tab === "sfu" && <SfuTab />}
        {tab === "federation" && <FederationBatchTab />}
        {tab === "quality" && <QualityTab />}
        {tab === "metrics" && <MetricsTab />}
      </div>
    </div>
  );
}

function ScalingTab() {
  const [configs, setConfigs] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_scaling_configs").then(setConfigs).catch(console.error);
  }, []);
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Scaling Configs</h3>
      {configs.length === 0 ? (
        <p className="text-gray-400 text-sm">No scaling configs yet.</p>
      ) : (
        <div className="space-y-2">
          {configs.map((c: any) => (
            <div key={c.id} className="bg-gray-800 rounded p-3 text-sm">
              <div className="flex justify-between">
                <span className="text-white font-medium">{c.instance_id}</span>
                <span className={clsx("text-xs px-2 py-0.5 rounded", c.is_active ? "bg-green-900 text-green-300" : "bg-gray-700 text-gray-400")}>
                  {c.is_active ? "Active" : "Inactive"}
                </span>
              </div>
              <div className="text-gray-400 mt-1">Region: {c.region} · Strategy: {c.shard_strategy} · Redis: {c.redis_mode}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SfuTab() {
  const [nodes, setNodes] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_sfu_nodes").then(setNodes).catch(console.error);
  }, []);
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">SFU Nodes</h3>
      {nodes.length === 0 ? (
        <p className="text-gray-400 text-sm">No SFU nodes registered.</p>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
          {nodes.map((n: any) => (
            <div key={n.id} className="bg-gray-800 rounded p-3 text-sm">
              <div className="font-medium text-white">{n.hostname}:{n.port}</div>
              <div className="text-gray-400">Region: {n.region} · Load: {n.current_load}/{n.capacity} · Status: {n.status}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function FederationBatchTab() {
  const [batches, setBatches] = useState<any[]>([]);
  const [routes, setRoutes] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_federation_batches").then(setBatches).catch(console.error);
    invoke("list_federation_routes").then(setRoutes).catch(console.error);
  }, []);
  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-white">Pending Batches</h3>
        {batches.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No pending batches.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {batches.map((b: any) => (
              <div key={b.id} className="bg-gray-800 rounded p-3 text-sm text-gray-300">
                Target: {b.target_instance} · Events: {b.event_count} · Status: {b.status}
              </div>
            ))}
          </div>
        )}
      </div>
      <div>
        <h3 className="text-lg font-semibold text-white">Federation Routes</h3>
        {routes.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No routes configured.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {routes.map((r: any) => (
              <div key={r.id} className="bg-gray-800 rounded p-3 text-sm text-gray-300">
                {r.source_instance} → {r.target_instance} · {r.latency_ms}ms · Priority: {r.priority}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function QualityTab() {
  return (
    <div className="space-y-3">
      <h3 className="text-lg font-semibold text-white">Voice Quality Logs</h3>
      <p className="text-gray-400 text-sm">Voice quality monitoring is available per-channel. Select a voice channel to view logs.</p>
    </div>
  );
}

function MetricsTab() {
  const [upgrades, setUpgrades] = useState<any[]>([]);
  useEffect(() => {
    invoke("list_upgrade_records").then(setUpgrades).catch(console.error);
  }, []);
  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-lg font-semibold text-white">Upgrade Records</h3>
        {upgrades.length === 0 ? (
          <p className="text-gray-400 text-sm mt-1">No upgrade records.</p>
        ) : (
          <div className="space-y-2 mt-2">
            {upgrades.map((u: any) => (
              <div key={u.id} className="bg-gray-800 rounded p-3 text-sm text-gray-300">
                {u.from_version} → {u.to_version} · Status: {u.status}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

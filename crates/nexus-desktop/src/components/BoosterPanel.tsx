/**
 * BoosterPanel — server booster management tab for ServerSettingsModal.
 * Shows tier progress, active boosters, boost/unboost controls, and vanity URL setter.
 *
 * v0.15 Community Ecosystem
 */
import { useEffect, useState } from "react";
import { useStore } from "../store";
import { invoke } from "../invoke";
import clsx from "clsx";

interface Props {
  serverId: string;
}

const TIER_THRESHOLDS = [0, 2, 7, 14] as const;

const TIER_COLORS: Record<number, string> = {
  0: "text-muted",
  1: "text-purple-400",
  2: "text-indigo-400",
  3: "text-accent-400",
};

const TIER_BG: Record<number, string> = {
  0: "bg-muted/20",
  1: "bg-purple-500/20",
  2: "bg-indigo-500/20",
  3: "bg-accent-500/20",
};

function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(0)} GB`;
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(0)} MB`;
  return `${(bytes / 1024).toFixed(0)} KB`;
}

export default function BoosterPanel({ serverId }: Props) {
  const { boostTierInfo, loadBoostTierInfo, session } = useStore();
  const info = boostTierInfo[serverId];

  const [boosters, setBoosters] = useState<
    { id: string; userId: string; slot: number; startedAt: string; username?: string }[]
  >([]);
  const [loadingBoosters, setLoadingBoosters] = useState(false);
  const [boosting, setBoosting] = useState(false);
  const [boostError, setBoostError] = useState<string | null>(null);
  const [vanityInput, setVanityInput] = useState("");
  const [vanityStatus, setVanityStatus] = useState<"idle" | "saving" | "ok" | "error">("idle");
  const [vanityError, setVanityError] = useState<string | null>(null);

  // Load tier info and boosters on mount
  useEffect(() => {
    loadBoostTierInfo(serverId);
    setLoadingBoosters(true);
    invoke<{ id: string; userId: string; slot: number; startedAt: string; username?: string }[]>(
      "list_server_boosters", { serverId }
    )
      .then(setBoosters)
      .catch(() => {})
      .finally(() => setLoadingBoosters(false));
  }, [serverId, loadBoostTierInfo]);

  // Sync vanity input with current value
  useEffect(() => {
    if (info?.currentVanityCode) setVanityInput(info.currentVanityCode);
  }, [info?.currentVanityCode]);

  const currentTier = info?.tier ?? 0;
  const boosterCount = info?.boosterCount ?? 0;
  const nextThreshold = TIER_THRESHOLDS[Math.min(currentTier + 1, 3)];
  const prevThreshold = TIER_THRESHOLDS[currentTier];
  const progressPct =
    currentTier >= 3
      ? 100
      : Math.min(100, ((boosterCount - prevThreshold) / (nextThreshold - prevThreshold)) * 100);

  const myBoostSlots = boosters.filter((b) => b.userId === session?.userId);
  const canBoost = myBoostSlots.length < 2;

  async function handleBoost() {
    setBoosting(true);
    setBoostError(null);
    try {
      await invoke("boost_server", { serverId });
      await loadBoostTierInfo(serverId);
      const updated = await invoke<typeof boosters>("list_server_boosters", { serverId });
      setBoosters(updated);
    } catch (e: unknown) {
      setBoostError(e instanceof Error ? e.message : "Failed to boost server");
    } finally {
      setBoosting(false);
    }
  }

  async function handleUnboost(slot: number) {
    setBoosting(true);
    setBoostError(null);
    try {
      await invoke("remove_boost", { serverId, slot });
      await loadBoostTierInfo(serverId);
      const updated = await invoke<typeof boosters>("list_server_boosters", { serverId });
      setBoosters(updated);
    } catch (e: unknown) {
      setBoostError(e instanceof Error ? e.message : "Failed to remove boost");
    } finally {
      setBoosting(false);
    }
  }

  async function handleSaveVanity() {
    setVanityStatus("saving");
    setVanityError(null);
    try {
      await invoke("set_vanity_url", { serverId, vanityCode: vanityInput.trim() });
      await loadBoostTierInfo(serverId);
      setVanityStatus("ok");
      setTimeout(() => setVanityStatus("idle"), 2500);
    } catch (e: unknown) {
      setVanityStatus("error");
      setVanityError(e instanceof Error ? e.message : "Failed to set vanity URL");
    }
  }

  return (
    <div className="space-y-5">
      {/* Tier card */}
      <div className={clsx("rounded-xl p-4 border", TIER_BG[currentTier], "border-bg-600/40")}>
        <div className="flex items-center justify-between mb-1">
          <span className={clsx("font-bold text-sm", TIER_COLORS[currentTier])}>
            {currentTier === 0 ? "No Tier" : `Tier ${currentTier}`}
          </span>
          <span className="text-xs text-muted">
            {boosterCount} booster{boosterCount !== 1 ? "s" : ""}
          </span>
        </div>

        {/* Progress bar */}
        {currentTier < 3 && (
          <div className="space-y-1">
            <div className="h-1.5 rounded-full bg-bg-600/60 overflow-hidden">
              <div
                className={clsx("h-full rounded-full transition-all duration-500", {
                  "bg-purple-500": currentTier === 0,
                  "bg-indigo-500": currentTier === 1,
                  "bg-accent-500": currentTier === 2,
                })}
                style={{ width: `${progressPct}%` }}
              />
            </div>
            <p className="text-[10px] text-muted text-right">
              {boosterCount} / {nextThreshold} boosts to Tier {currentTier + 1}
            </p>
          </div>
        )}
        {currentTier >= 3 && (
          <p className="text-[10px] text-accent-400 mt-0.5">Maximum tier reached 🎉</p>
        )}
      </div>

      {/* Perks */}
      {info && (
        <div className="rounded-lg border border-bg-600/40 bg-bg-700/40 p-3 space-y-2">
          <p className="text-[10px] uppercase tracking-widest text-muted/70 font-medium">Active Perks</p>
          <div className="flex flex-wrap gap-2 text-xs text-fg/80">
            <span className="bg-bg-600/60 rounded px-2 py-0.5">
              +{info.extraEmojiSlots} emoji slots
            </span>
            <span className="bg-bg-600/60 rounded px-2 py-0.5">
              {formatBytes(info.uploadLimitBytes)} upload limit
            </span>
            {info.vanityUrlAvailable && (
              <span className="bg-indigo-900/40 text-indigo-300 rounded px-2 py-0.5">
                Vanity URL available
              </span>
            )}
          </div>
        </div>
      )}

      {/* Vanity URL setter — only at tier 2+ */}
      {info?.vanityUrlAvailable && (
        <div className="space-y-2">
          <p className="text-sm font-medium text-fg">Vanity URL</p>
          <div className="flex gap-2">
            <div className="flex-1 flex items-center gap-1 rounded-lg border border-bg-600/50 bg-bg-700 px-3 py-2">
              <span className="text-muted text-xs shrink-0">nexus.gg/</span>
              <input
                value={vanityInput}
                onChange={(e) => { setVanityInput(e.target.value); setVanityStatus("idle"); }}
                placeholder="your-server"
                className="flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-muted/60"
                maxLength={32}
              />
            </div>
            <button
              onClick={handleSaveVanity}
              disabled={!vanityInput.trim() || vanityStatus === "saving"}
              className="rounded-lg bg-accent-600 hover:bg-accent-500 disabled:opacity-40 text-white text-sm px-4 py-2 font-medium transition-colors"
            >
              {vanityStatus === "saving" ? "Saving…" : "Save"}
            </button>
          </div>
          {vanityStatus === "ok" && <p className="text-xs text-green-400">✓ Vanity URL saved</p>}
          {vanityStatus === "error" && <p className="text-xs text-red-400">{vanityError}</p>}
        </div>
      )}

      {/* Boost button */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <p className="text-sm font-medium text-fg">Your Boosts</p>
          {canBoost && (
            <button
              onClick={handleBoost}
              disabled={boosting}
              className="text-xs bg-purple-700 hover:bg-purple-600 text-white rounded-lg px-3 py-1.5 font-medium transition-colors disabled:opacity-50"
            >
              {boosting ? "Boosting…" : "💎 Boost Server"}
            </button>
          )}
        </div>
        {boostError && <p className="text-xs text-red-400">{boostError}</p>}
        {myBoostSlots.length > 0 ? (
          <div className="flex flex-col gap-1">
            {myBoostSlots.map((b) => (
              <div key={b.id} className="flex items-center justify-between rounded-lg bg-purple-900/20 border border-purple-700/30 px-3 py-2 text-xs text-fg/80">
                <span>Slot {b.slot} · Boosting since {new Date(b.startedAt).toLocaleDateString()}</span>
                <button
                  onClick={() => handleUnboost(b.slot)}
                  disabled={boosting}
                  className="text-red-400 hover:text-red-300 transition-colors disabled:opacity-50"
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-xs text-muted">You are not boosting this server.</p>
        )}
      </div>

      {/* All boosters list */}
      <div className="space-y-2">
        <p className="text-sm font-medium text-fg">
          All Boosters
          <span className="ml-2 text-xs text-muted font-normal">({boosters.length})</span>
        </p>
        {loadingBoosters ? (
          <p className="text-xs text-muted">Loading…</p>
        ) : boosters.length === 0 ? (
          <p className="text-xs text-muted">No boosters yet. Be the first!</p>
        ) : (
          <div className="flex flex-col gap-1 max-h-48 overflow-y-auto">
            {boosters.map((b) => (
              <div key={b.id} className="flex items-center gap-2 rounded-lg bg-bg-700/40 px-3 py-1.5 text-xs text-fg/80">
                <span className="text-purple-400">💎</span>
                <span className="flex-1 truncate">{b.username ?? b.userId}</span>
                <span className="text-muted shrink-0">Slot {b.slot}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

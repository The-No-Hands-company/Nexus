import { useEffect, useMemo } from "react";
import { useStore } from "../store";
import type { AnalyticsSnapshot } from "../store";

/**
 * AdminAnalyticsPanel — Server analytics dashboard showing growth trends,
 * activity, and moderation stats over the last N days.
 */
export default function AdminAnalyticsPanel({ serverId }: { serverId: string }) {
  const { analyticsSnapshots, loadAnalyticsSnapshots } = useStore();
  const snaps = analyticsSnapshots[serverId] ?? [];

  useEffect(() => {
    loadAnalyticsSnapshots(serverId, 30);
  }, [serverId, loadAnalyticsSnapshots]);

  // Aggregate summaries
  const summary = useMemo(() => {
    if (snaps.length === 0)
      return {
        totalMessages: 0,
        avgActive: 0,
        totalNew: 0,
        totalLeft: 0,
        totalVoiceMinutes: 0,
        totalReports: 0,
        totalBans: 0,
      };
    return {
      totalMessages: snaps.reduce((s, r) => s + r.messagesCount, 0),
      avgActive: Math.round(snaps.reduce((s, r) => s + r.activeMembers, 0) / snaps.length),
      totalNew: snaps.reduce((s, r) => s + r.newMembers, 0),
      totalLeft: snaps.reduce((s, r) => s + r.leftMembers, 0),
      totalVoiceMinutes: snaps.reduce((s, r) => s + r.voiceMinutes, 0),
      totalReports: snaps.reduce((s, r) => s + r.reportsResolved, 0),
      totalBans: snaps.reduce((s, r) => s + r.bansIssued, 0),
    };
  }, [snaps]);

  // Simple bar chart helper
  const maxMessages = Math.max(1, ...snaps.map((s) => s.messagesCount));

  const StatCard = ({ label, value, sub }: { label: string; value: string | number; sub?: string }) => (
    <div className="bg-gray-800/60 rounded-lg p-3 flex flex-col">
      <span className="text-xs text-gray-500">{label}</span>
      <span className="text-xl font-bold text-white">{value}</span>
      {sub && <span className="text-[10px] text-gray-600">{sub}</span>}
    </div>
  );

  return (
    <div className="flex flex-col gap-5 p-4 max-w-3xl">
      <h2 className="text-lg font-bold text-white">Server Analytics</h2>
      <p className="text-sm text-gray-400">Activity overview for the last 30 days.</p>

      {snaps.length === 0 ? (
        <p className="text-sm text-gray-600">No analytics data yet.</p>
      ) : (
        <>
          {/* Summary cards */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <StatCard label="Messages" value={summary.totalMessages.toLocaleString()} />
            <StatCard label="Avg Active Members" value={summary.avgActive} />
            <StatCard label="New Members" value={summary.totalNew} />
            <StatCard label="Left Members" value={summary.totalLeft} />
            <StatCard label="Voice Minutes" value={summary.totalVoiceMinutes.toLocaleString()} />
            <StatCard label="Reports Resolved" value={summary.totalReports} />
            <StatCard label="Bans Issued" value={summary.totalBans} />
            <StatCard label="Data Points" value={snaps.length} sub="days of data" />
          </div>

          {/* Message activity chart */}
          <div>
            <h3 className="text-sm font-semibold text-gray-300 mb-2">Daily Messages</h3>
            <div className="flex items-end gap-px h-32 bg-gray-900/50 rounded p-2">
              {snaps.map((snap) => {
                const height = Math.max(2, (snap.messagesCount / maxMessages) * 100);
                return (
                  <div
                    key={snap.id}
                    className="flex-1 bg-indigo-500/80 rounded-t hover:bg-indigo-400 transition-colors"
                    style={{ height: `${height}%` }}
                    title={`${snap.periodDate}: ${snap.messagesCount} messages`}
                  />
                );
              })}
            </div>
            <div className="flex justify-between mt-1">
              <span className="text-[10px] text-gray-600">{snaps[0]?.periodDate}</span>
              <span className="text-[10px] text-gray-600">{snaps[snaps.length - 1]?.periodDate}</span>
            </div>
          </div>

          {/* Member growth chart */}
          <div>
            <h3 className="text-sm font-semibold text-gray-300 mb-2">Member Growth</h3>
            <div className="space-y-1">
              {snaps.slice(-7).map((snap) => (
                <div key={snap.id} className="flex items-center gap-2 text-xs">
                  <span className="text-gray-500 w-20 text-right">{snap.periodDate.slice(5)}</span>
                  <span className="text-green-400 w-10 text-right">+{snap.newMembers}</span>
                  <span className="text-red-400 w-10 text-right">-{snap.leftMembers}</span>
                  <span className="text-gray-600">
                    net {snap.newMembers - snap.leftMembers >= 0 ? "+" : ""}
                    {snap.newMembers - snap.leftMembers}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}

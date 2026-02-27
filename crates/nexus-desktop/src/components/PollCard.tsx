/**
 * PollCard — renders an inline poll inside the chat message list.
 *
 * Features:
 * - Animated vote-bar fill (width animates on every render)
 * - Real-time countdown timer for polls with a deadline
 * - Click to cast / retract a vote (multiselect-aware)
 * - "Ended" state with locked options
 * - Respects `is_anonymous` (hides voter counts when set)
 * - Fetches full results from the server on mount; refreshes via store when
 *   the gateway fires POLL_VOTE_ADD / POLL_VOTE_REMOVE
 */

import { useEffect, useState, useCallback } from "react";
import clsx from "clsx";
import { useStore } from "../store";
import type { Poll, PollResults } from "../store";
import { invoke } from "../invoke";

interface PollCardProps {
  poll: Poll;
  channelId: string;
  currentUserId: string;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatCountdown(endsAt: string): string {
  const delta = Math.max(0, new Date(endsAt).getTime() - Date.now());
  const totalSecs = Math.floor(delta / 1000);
  const days = Math.floor(totalSecs / 86400);
  const hours = Math.floor((totalSecs % 86400) / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;

  if (days > 0) return `${days}d ${hours}h remaining`;
  if (hours > 0) return `${hours}h ${mins}m remaining`;
  if (mins > 0) return `${mins}m ${secs}s remaining`;
  if (secs > 0) return `${secs}s remaining`;
  return "Ending…";
}

// ── Component ─────────────────────────────────────────────────────────────────

export default function PollCard({ poll, channelId, currentUserId }: PollCardProps) {
  const { pollResults, setPollResults } = useStore();
  const results: PollResults | undefined = pollResults[poll.id];

  const [votedIndices, setVotedIndices] = useState<Set<number>>(new Set());
  const [voting, setVoting] = useState(false);
  const [countdown, setCountdown] = useState<string>("");
  const [loadError, setLoadError] = useState(false);

  const isEnded = poll.status === "ended";

  // ── Load results on mount ──────────────────────────────────────────────────
  useEffect(() => {
    if (results) return; // already cached
    invoke<{ poll: Poll; options: { index: number; label: string; vote_count: number; voter_ids: string[] }[]; total_voters: number }>(
      "get_poll_results",
      { channelId, pollId: poll.id }
    )
      .then((res) => {
        setPollResults(poll.id, {
          poll: res.poll,
          options: res.options.map((o) => ({
            index: o.index,
            label: o.label,
            voteCount: o.vote_count,
            voterIds: o.voter_ids,
          })),
          totalVoters: res.total_voters,
        });
        // Derive which options the current user voted on
        const voted = new Set<number>();
        for (const opt of res.options) {
          if (opt.voter_ids?.includes(currentUserId)) voted.add(opt.index);
        }
        setVotedIndices(voted);
      })
      .catch(() => setLoadError(true));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [poll.id, channelId]);

  // Update voted state when results change (e.g. gateway refresh)
  useEffect(() => {
    if (!results) return;
    const voted = new Set<number>();
    for (const opt of results.options) {
      if (opt.voterIds?.includes(currentUserId)) voted.add(opt.index);
    }
    setVotedIndices(voted);
  }, [results, currentUserId]);

  // ── Countdown timer ────────────────────────────────────────────────────────
  useEffect(() => {
    if (!poll.endsAt || isEnded) return;
    const tick = () => setCountdown(formatCountdown(poll.endsAt!));
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [poll.endsAt, isEnded]);

  // ── Voting ─────────────────────────────────────────────────────────────────
  const handleVote = useCallback(
    async (optionIndex: number) => {
      if (isEnded || voting) return;
      setVoting(true);
      try {
        const alreadyVoted = votedIndices.has(optionIndex);
        if (alreadyVoted) {
          await invoke("retract_poll_vote", { channelId, pollId: poll.id });
          setVotedIndices(new Set());
        } else {
          const indices = poll.allowMultiselect
            ? [...votedIndices, optionIndex]
            : [optionIndex];
          await invoke("cast_poll_vote", { channelId, pollId: poll.id, optionIndices: indices });
          setVotedIndices(new Set(indices));
        }
        // Refresh results
        const res = await invoke<{
          poll: Poll;
          options: { index: number; label: string; vote_count: number; voter_ids: string[] }[];
          total_voters: number;
        }>("get_poll_results", { channelId, pollId: poll.id });
        setPollResults(poll.id, {
          poll: res.poll,
          options: res.options.map((o) => ({
            index: o.index,
            label: o.label,
            voteCount: o.vote_count,
            voterIds: o.voter_ids,
          })),
          totalVoters: res.total_voters,
        });
      } catch (e) {
        console.error("[PollCard] vote error", e);
      } finally {
        setVoting(false);
      }
    },
    [isEnded, voting, votedIndices, poll, channelId, setPollResults]
  );

  // ── Render ─────────────────────────────────────────────────────────────────
  const totalVotes = results?.totalVoters ?? 0;

  return (
    <div className="rounded-lg border border-bg-600/50 bg-bg-700 p-4 my-1 max-w-md w-full select-none">
      {/* Header row */}
      <div className="flex items-start justify-between gap-2 mb-3">
        <p className="font-semibold text-fg text-sm leading-snug">{poll.question}</p>
        <div className="flex items-center gap-1.5 shrink-0">
          {isEnded ? (
            <span className="text-xs bg-dnd/20 text-dnd rounded px-1.5 py-0.5 font-medium">
              Ended
            </span>
          ) : (
            countdown && (
              <span className="text-xs text-muted">⏳ {countdown}</span>
            )
          )}
          {poll.isAnonymous && (
            <span className="text-xs text-muted/70 bg-bg-600 rounded px-1.5 py-0.5">
              anon
            </span>
          )}
        </div>
      </div>

      {/* Options */}
      {loadError ? (
        <p className="text-xs text-muted italic">Could not load poll results.</p>
      ) : (
        <div className="flex flex-col gap-1.5">
          {(results?.options ?? poll.options.map((label, index) => ({ index, label, voteCount: 0, voterIds: [] }))).map((opt) => {
            const pct = totalVotes > 0 ? Math.round((opt.voteCount / totalVotes) * 100) : 0;
            const voted = votedIndices.has(opt.index);

            return (
              <button
                key={opt.index}
                disabled={isEnded || voting}
                onClick={() => handleVote(opt.index)}
                className={clsx(
                  "relative w-full flex items-center justify-between px-3 py-2 rounded text-sm text-left",
                  "transition-all duration-150 overflow-hidden",
                  voted
                    ? "ring-1 ring-accent-400/60 text-accent-400"
                    : "text-fg hover:ring-1 hover:ring-accent-400/30",
                  (isEnded || voting) ? "cursor-default" : "cursor-pointer"
                )}
              >
                {/* Fill bar */}
                <span
                  className={clsx(
                    "absolute inset-y-0 left-0 rounded transition-all duration-500",
                    voted ? "bg-accent-400/20" : "bg-bg-600/60"
                  )}
                  style={{ width: `${pct}%` }}
                  aria-hidden="true"
                />
                {/* Label */}
                <span className="relative z-10 truncate">{opt.label}</span>
                {/* Vote info */}
                <span className="relative z-10 text-xs text-muted shrink-0 ml-2">
                  {results
                    ? poll.isAnonymous && !isEnded
                      ? `${pct}%`
                      : `${opt.voteCount} • ${pct}%`
                    : null}
                </span>
              </button>
            );
          })}
        </div>
      )}

      {/* Footer */}
      <p className="text-xs text-muted mt-2.5">
        {totalVotes === 1 ? "1 vote" : `${totalVotes} votes`}
        {poll.allowMultiselect && " · multi-select"}
      </p>
    </div>
  );
}

/**
 * StreamView — topic-threaded message view for stream channels.
 *
 * When a channel has `isStream = true`, messages are grouped by their `topic`
 * field and displayed as collapsible topic sections instead of a flat timeline.
 *
 * Messages without a topic appear in an "Untopiced" section.
 */

import { useState, useMemo } from "react";
import clsx from "clsx";
import { useStore, Message } from "../store";

interface Props {
  channelId: string;
  /** Called when the user wants to send a message with a topic. */
  onReply?: (topic: string) => void;
}

interface TopicGroup {
  topic: string;
  messages: Message[];
  latest: string;
}

export default function StreamView({ channelId, onReply }: Props) {
  const { messages: allMessages } = useStore();
  const channelMsgs = allMessages[channelId] ?? [];

  const [expandedTopics, setExpandedTopics] = useState<Set<string>>(new Set([""]));
  const [newTopic, setNewTopic] = useState("");

  // Group messages by topic
  const groups = useMemo<TopicGroup[]>(() => {
    const map = new Map<string, Message[]>();
    for (const msg of channelMsgs) {
      const t = msg.topic ?? "";
      if (!map.has(t)) map.set(t, []);
      map.get(t)!.push(msg);
    }
    return Array.from(map.entries())
      .map(([topic, msgs]) => ({
        topic,
        messages: msgs,
        latest: msgs[msgs.length - 1]?.createdAt ?? "",
      }))
      .sort((a, b) => b.latest.localeCompare(a.latest));
  }, [channelMsgs]);

  const toggleTopic = (t: string) => {
    setExpandedTopics((prev) => {
      const next = new Set(prev);
      if (next.has(t)) next.delete(t);
      else next.add(t);
      return next;
    });
  };

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* New topic input */}
      <div className="flex items-center gap-2 border-b border-bg-600/40 px-4 py-2">
        <input
          value={newTopic}
          onChange={(e) => setNewTopic(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && newTopic.trim()) {
              onReply?.(newTopic.trim());
              setNewTopic("");
            }
          }}
          placeholder="Start a new topic… (press Enter)"
          maxLength={80}
          className="flex-1 rounded-lg bg-bg-700 px-3 py-1.5 text-sm text-fg placeholder-muted outline-none focus:ring-1 focus:ring-brand"
        />
        <span className="text-xs text-muted">{newTopic.length}/80</span>
      </div>

      {/* Topic groups */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {groups.length === 0 && (
          <p className="py-8 text-center text-sm text-muted">
            No messages yet. Start a topic above.
          </p>
        )}
        {groups.map((group) => {
          const label = group.topic || "General";
          const expanded = expandedTopics.has(group.topic);
          return (
            <div key={group.topic} className="rounded-xl border border-bg-600/30 bg-bg-750 overflow-hidden">
              {/* Topic header */}
              <button
                onClick={() => toggleTopic(group.topic)}
                className="flex w-full items-center justify-between px-4 py-2.5 hover:bg-bg-700/50 transition-colors"
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm font-semibold text-fg">{label}</span>
                  <span className="rounded-full bg-brand/20 px-1.5 py-0.5 text-[10px] font-medium text-brand">
                    {group.messages.length}
                  </span>
                </div>
                <span className={clsx("text-muted text-xs transition-transform", expanded && "rotate-180")}>
                  ▼
                </span>
              </button>

              {/* Messages */}
              {expanded && (
                <div className="border-t border-bg-600/30 divide-y divide-bg-600/20">
                  {group.messages.map((msg) => (
                    <div key={msg.id} className="flex gap-3 px-4 py-2.5 hover:bg-bg-700/30">
                      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-bg-600 text-xs font-bold uppercase text-fg">
                        {msg.authorUsername.charAt(0)}
                      </div>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-baseline gap-2">
                          <span className="text-xs font-semibold text-fg">{msg.authorUsername}</span>
                          <span className="text-[10px] text-muted">
                            {new Date(msg.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                          </span>
                        </div>
                        <p className="mt-0.5 text-sm text-fg/90 break-words">{msg.content}</p>
                      </div>
                      <button
                        onClick={() => onReply?.(group.topic)}
                        className="shrink-0 rounded px-2 py-1 text-[10px] font-medium text-muted hover:bg-bg-600/60 hover:text-fg"
                        title={`Reply in "${label}"`}
                      >
                        Reply
                      </button>
                    </div>
                  ))}
                  {/* Reply in topic */}
                  <button
                    onClick={() => onReply?.(group.topic)}
                    className="w-full px-4 py-2 text-left text-xs text-muted hover:bg-bg-700/30 hover:text-fg transition-colors"
                  >
                    + Post in "{label}"
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

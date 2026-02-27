import { useEffect, useRef, useCallback, useState } from "react";
import { useParams } from "react-router-dom";
import { useStore, Message } from "../store";
import type { Embed } from "../store";
import { invoke } from "../invoke";
import MessageInput from "./MessageInput";
import MemberList from "./MemberList";
import ThreadPanel from "./ThreadPanel";
import PollCard from "./PollCard";
import SavedMessagesPanel from "./SavedMessagesPanel";
import ForwardModal from "./ForwardModal";
import EventsPanel from "./EventsPanel";
import StreamView from "./StreamView";
import CanvasView from "./CanvasView";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { formatDistanceToNow, format, isToday, isYesterday } from "date-fns";
import clsx from "clsx";

export default function ChatView() {
  const { channelId } = useParams<{ channelId: string }>();
  const {
    messages, channels, session, loadMessages, activeServerId, isHomeMode,
    channelPolls, setChannelPolls, addBookmark, savedMessagesPanelOpen, setSavedMessagesPanelOpen,
    forwardModalMessage, setForwardModalMessage, eventsOpen, setEventsOpen,
  } = useStore();
  const [showMembers, setShowMembers] = useState(true);
  const [openThread, setOpenThread] = useState<{ id: string; title: string } | null>(null);
  const [startingThread, setStartingThread] = useState<{ msgId: string; msgContent: string } | null>(null);
  const [newThreadTitle, setNewThreadTitle] = useState("");
  const [showScheduled, setShowScheduled] = useState(false);
  const [scheduledMsgs, setScheduledMsgs] = useState<import("../store").ScheduledMessage[]>([]);
  const [confirmDisappear, setConfirmDisappear] = useState(false);
  const [pendingTopic, setPendingTopic] = useState<string | undefined>(undefined);

  const msgs: Message[] = channelId ? (messages[channelId] ?? []) : [];
  const channel = channels.find((c) => c.id === channelId);
  const polls = channelId ? (channelPolls[channelId] ?? []) : [];

  // Build a unified timeline of messages and standalone polls sorted by createdAt
  type TimelineItem =
    | { type: "message"; data: Message }
    | { type: "poll"; data: import("../store").Poll };

  const timeline: TimelineItem[] = [
    ...msgs.map((m) => ({ type: "message" as const, data: m })),
    ...polls
      .filter((p) => !p.messageId) // standalone polls only; attached polls render inside MessageRow
      .map((p) => ({ type: "poll" as const, data: p })),
  ].sort((a, b) => new Date(a.data.createdAt).getTime() - new Date(b.data.createdAt).getTime());
  const bottomRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const startThreadTrapRef = useFocusTrap<HTMLDivElement>(!!startingThread);

  useEffect(() => {
    if (channelId) {
      loadMessages(channelId);
    }
  }, [channelId, loadMessages]);

  // Load polls for the active channel
  useEffect(() => {
    if (!channelId) return;
    invoke<import("../store").Poll[]>("list_channel_polls", { channelId })
      .then((polls) => setChannelPolls(channelId, polls))
      .catch(() => {}); // best-effort — polls section just won't render
  }, [channelId, setChannelPolls]);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [msgs.length]);

  // Load older messages on scroll to top
  const handleScroll = useCallback(() => {
    const el = containerRef.current;
    if (!el || !channelId) return;
    if (el.scrollTop < 80 && msgs.length > 0) {
      loadMessages(channelId, msgs[0]?.id);
    }
  }, [channelId, msgs, loadMessages]);

  if (!channelId) return null;

  // Only show member list for server channels, not DMs
  const isServerChannel = !!activeServerId && !isHomeMode;

  return (
    <div className="flex h-full overflow-hidden bg-bg-800">
      {/* Chat column */}
      <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
        {/* Channel header */}
        <div className="h-12 px-4 flex items-center gap-2 border-b border-bg-600/50 shrink-0 no-select">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" className="text-muted shrink-0">
            <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm-2 12H6v-2h12v2zm0-3H6V9h12v2zm0-3H6V6h12v2z"/>
          </svg>
          <span className="font-semibold text-fg text-sm flex-1">{channel?.name}</span>
          {/* Disappearing messages timer */}
          {channel?.disappearAfterSeconds != null && channel.disappearAfterSeconds > 0 && (
            <button
              title={`Disappearing messages: ${formatDisappearTimer(channel.disappearAfterSeconds)}`}
              onClick={() => setConfirmDisappear(true)}
              className="text-xs text-muted hover:text-fg transition-colors flex items-center gap-1"
            >
              ⏳ {formatDisappearTimer(channel.disappearAfterSeconds)}
            </button>
          )}
          {/* Schedule Messages indicator */}
          <div className="relative">
            <button
              onClick={async () => {
                if (!channelId) return;
                if (!showScheduled) {
                  const msgs = await invoke<import("../store").ScheduledMessage[]>(
                    "list_scheduled_messages", { channelId }
                  ).catch(() => []);
                  setScheduledMsgs(msgs);
                }
                setShowScheduled((v) => !v);
              }}
              title="Scheduled messages"
              className={clsx(
                "p-1.5 rounded transition-colors text-sm",
                showScheduled ? "text-accent-400 bg-bg-600/60" : "text-muted hover:text-fg hover:bg-bg-700/60"
              )}
            >
              🗓️
            </button>
            {showScheduled && (
              <div className="absolute right-0 top-full mt-1 z-20 w-80 rounded-lg border border-bg-600/50 bg-bg-700 shadow-xl p-3 text-sm">
                <p className="text-[10px] uppercase tracking-widest text-muted mb-2">Scheduled Messages</p>
                {scheduledMsgs.length === 0 ? (
                  <p className="text-xs text-muted text-center py-4">No scheduled messages</p>
                ) : (
                  <div className="flex flex-col gap-2 max-h-64 overflow-y-auto">
                    {scheduledMsgs.map((sm) => (
                      <div key={sm.id} className="flex items-start justify-between gap-2 rounded bg-bg-600/40 p-2">
                        <div className="min-w-0">
                          <p className="text-xs text-fg truncate">{sm.content}</p>
                          <p className="text-[10px] text-muted mt-0.5">
                            {format(new Date(sm.scheduledAt), "MMM d, HH:mm")}
                          </p>
                        </div>
                        <button
                          onClick={async () => {
                            await invoke("cancel_scheduled_message", { channelId, scheduledMessageId: sm.id }).catch(() => null);
                            setScheduledMsgs((prev) => prev.filter((m) => m.id !== sm.id));
                          }}
                          className="text-[10px] text-dnd hover:text-red-400 shrink-0"
                        >
                          Cancel
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
          {channel?.isE2ee && (
            <span className="text-xs bg-green-900/40 text-green-400 px-1.5 py-0.5 rounded font-medium">
              E2EE
            </span>
          )}
          {isServerChannel && (
            <button
              onClick={() => setShowMembers((v) => !v)}
              title={showMembers ? "Hide members" : "Show members"}
              aria-label={showMembers ? "Hide member list" : "Show member list"}
              aria-pressed={showMembers}
              className={clsx(
                "p-1.5 rounded transition-colors",
                showMembers ? "text-fg bg-bg-600/60" : "text-muted hover:text-fg hover:bg-bg-700/60"
              )}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                <path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z"/>
              </svg>
            </button>
          )}
          {/* Events button — v0.14 Platform Differentiation */}
          {isServerChannel && activeServerId && (
            <button
              onClick={() => setEventsOpen(!eventsOpen)}
              title={eventsOpen ? "Hide events" : "Show events"}
              aria-label="Toggle server events panel"
              aria-pressed={eventsOpen}
              className={clsx(
                "p-1.5 rounded transition-colors text-sm leading-none",
                eventsOpen ? "text-fg bg-bg-600/60" : "text-muted hover:text-fg hover:bg-bg-700/60"
              )}
            >
              🗓
            </button>
          )}
        </div>

        {/* Messages — Canvas channels get a block editor (v0.15), Stream channels get StreamView (v0.14) */}
        {channel?.kind === "canvas" ? (
          <CanvasView channelId={channelId!} />
        ) : channel?.isStream ? (
          <StreamView
            channelId={channelId!}
            onReply={(topic) => setPendingTopic(topic)}
          />
        ) : (
        <div
          ref={containerRef}
          onScroll={handleScroll}
          role="log"
          aria-live="polite"
          aria-label={channel ? `Messages in ${channel.name}` : "Messages"}
          aria-relevant="additions"
          className="nexus-msg-list flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-0.5"
        >
          {timeline.length === 0 && (
            <p className="text-muted text-sm text-center mt-16">
              No messages yet. Be the first!
            </p>
          )}
          {timeline.map((item, i) => {
            if (item.type === "poll") {
              return (
                <PollCard
                  key={`poll-${item.data.id}`}
                  poll={item.data}
                  channelId={channelId!}
                  currentUserId={session?.userId ?? ""}
                />
              );
            }

            const msg = item.data;
            const prevItem = timeline[i - 1];
            const prevMsg = prevItem?.type === "message" ? prevItem.data : null;
            const grouped =
              !!prevMsg &&
              prevMsg.authorId === msg.authorId &&
              new Date(msg.createdAt).getTime() -
                new Date(prevMsg.createdAt).getTime() <
                5 * 60 * 1000;
            // Date separator when the calendar day changes
            const newDay = !prevItem ||
              new Date(msg.createdAt).toDateString() !== new Date(prevItem.data.createdAt).toDateString();

            // Attached poll for this message (if any)
            const attachedPoll = polls.find((p) => p.messageId === msg.id);

            return (
              <>
                {newDay && <DateSeparator key={`sep-${msg.id}`} date={new Date(msg.createdAt)} />}
                <MessageRow
                  key={msg.id}
                  msg={msg}
                  channelId={channelId}
                  grouped={grouped && !newDay}
                  isOwn={msg.authorId === session?.userId}
                  onOpenThread={(tId, title) => { setOpenThread({ id: tId, title }); setShowMembers(false); }}
                  onStartThread={(msgId, content) => { setStartingThread({ msgId, msgContent: content }); setNewThreadTitle(""); }}
                  onBookmark={async () => {
                    try {
                      const bm = await invoke<import("../store").Bookmark>("add_bookmark", {
                        messageId: msg.id,
                        channelId,
                      });
                      addBookmark(bm);
                    } catch (e) {
                      console.error("[bookmark]", e);
                    }
                  }}
                  onForward={() => setForwardModalMessage(msg)}
                />
                {attachedPoll && (
                  <PollCard
                    key={`poll-attached-${attachedPoll.id}`}
                    poll={attachedPoll}
                    channelId={channelId!}
                    currentUserId={session?.userId ?? ""}
                  />
                )}
              </>
            );
          })}
          <div ref={bottomRef} />
        </div>
        )}

        {/* Typing bar — hidden for canvas channels (no chat) */}
        {channelId && channel?.kind !== "canvas" && <TypingBar channelId={channelId} />}

        {/* Input — hidden for canvas channels */}
        {channel?.kind !== "canvas" && (
          <MessageInput channelId={channelId} isE2ee={!!channel?.isE2ee} pendingTopic={pendingTopic} onTopicConsumed={() => setPendingTopic(undefined)} />
        )}
      </div>

      {/* Saved Messages panel (replaces member list when open) */}
      {savedMessagesPanelOpen && (
        <SavedMessagesPanel onClose={() => setSavedMessagesPanelOpen(false)} />
      )}

      {/* EventsPanel — v0.14 */}
      {eventsOpen && activeServerId && (
        <EventsPanel serverId={activeServerId} onClose={() => setEventsOpen(false)} />
      )}

      {/* ForwardModal — v0.14 */}
      {forwardModalMessage && (
        <ForwardModal message={forwardModalMessage} onClose={() => setForwardModalMessage(null)} />
      )}

      {/* Member list sidebar */}
      {isServerChannel && showMembers && !openThread && !savedMessagesPanelOpen && !eventsOpen && (
        <MemberList serverId={activeServerId!} />
      )}

      {/* Thread panel */}
      {openThread && (
        <ThreadPanel
          threadId={openThread.id}
          threadTitle={openThread.title}
          onClose={() => setOpenThread(null)}
        />
      )}

      {/* Start-thread overlay */}
      {startingThread && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div
            ref={startThreadTrapRef}
            role="dialog"
            aria-modal="true"
            aria-labelledby="start-thread-title"
            className="w-96 rounded-xl bg-bg-700 p-6 shadow-2xl"
          >
            <h2 id="start-thread-title" className="mb-1 text-base font-semibold text-fg">Start a thread</h2>
            <p className="mb-4 truncate text-xs text-muted">{startingThread.msgContent}</p>
            <label className="mb-1 block text-xs font-medium uppercase tracking-widest text-muted">
              Thread title
            </label>
            <input
              autoFocus
              value={newThreadTitle}
              onChange={(e) => setNewThreadTitle(e.target.value)}
              aria-label="Thread title"
              onKeyDown={async (e) => {
                if (e.key === "Enter" && newThreadTitle.trim() && channelId) {
                  e.preventDefault();
                  try {
                    const thread = await invoke<{ id: string; title: string }>(
                      "create_thread",
                      { channelId, title: newThreadTitle.trim() }
                    );
                    setStartingThread(null);
                    setOpenThread({ id: thread.id, title: thread.title });
                    setShowMembers(false);
                  } catch (err) {
                    console.error("create_thread error", err);
                  }
                } else if (e.key === "Escape") {
                  setStartingThread(null);
                }
              }}
              placeholder="e.g. Discussion about this post"
              className="mb-4 w-full rounded-lg border border-bg-600/50 bg-bg-800 px-3 py-2 text-sm text-fg placeholder:text-muted/60 outline-none focus:border-accent-500"
            />
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setStartingThread(null)}
                className="rounded-lg px-4 py-2 text-sm text-muted hover:text-fg transition-colors"
              >
                Cancel
              </button>
              <button
                disabled={!newThreadTitle.trim()}
                onClick={async () => {
                  if (!newThreadTitle.trim() || !channelId) return;
                  try {
                    const thread = await invoke<{ id: string; title: string }>(
                      "create_thread",
                      { channelId, title: newThreadTitle.trim() }
                    );
                    setStartingThread(null);
                    setOpenThread({ id: thread.id, title: thread.title });
                    setShowMembers(false);
                  } catch (err) {
                    console.error("create_thread error", err);
                  }
                }}
                className="rounded-lg bg-accent-600 px-4 py-2 text-sm font-medium text-white hover:bg-accent-500 disabled:opacity-40 transition-colors"
              >
                Create Thread
              </button>
            </div>
          </div>
        </div>
      )}
      {/* Confirm disappearing messages info */}
      {confirmDisappear && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="w-80 rounded-xl bg-bg-700 p-6 shadow-2xl">
            <h2 className="mb-2 text-base font-semibold text-fg">Disappearing Messages</h2>
            <p className="text-sm text-muted mb-4">
              Messages in this channel auto-delete after{" "}
              {channel ? formatDisappearTimer(channel.disappearAfterSeconds ?? 0) : "a set time"}.
              This setting is managed by the server admin.
            </p>
            <button
              onClick={() => setConfirmDisappear(false)}
              className="rounded-lg bg-bg-600 px-4 py-2 text-sm text-fg hover:bg-bg-500 transition-colors"
            >
              Got it
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Disappear timer helper ────────────────────────────────────────────────────
function formatDisappearTimer(secs: number): string {
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h`;
  return `${Math.floor(secs / 86400)}d`;
}

// ── Date separator ────────────────────────────────────────────────────────────
function DateSeparator({ date }: { date: Date }) {
  const label = isToday(date)
    ? "Today"
    : isYesterday(date)
    ? "Yesterday"
    : format(date, "MMMM d, yyyy");
  return (
    <div className="flex items-center gap-3 my-3 px-1 no-select">
      <div className="flex-1 h-px bg-bg-600/40" />
      <span className="text-[11px] text-muted/60 font-medium uppercase tracking-wider shrink-0">
        {label}
      </span>
      <div className="flex-1 h-px bg-bg-600/40" />
    </div>
  );
}

// ── Typing indicator bar ──────────────────────────────────────────────────────
export function TypingBar({ channelId }: { channelId: string }) {
  const typingUsers = useStore((s) => s.typingUsers[channelId] ?? []);
  if (typingUsers.length === 0) return null;

  let label: string;
  if (typingUsers.length === 1) label = `${typingUsers[0]} is typing…`;
  else if (typingUsers.length === 2) label = `${typingUsers[0]} and ${typingUsers[1]} are typing…`;
  else label = "Several people are typing…";

  return (
    <div className="px-4 pb-1 flex items-center gap-1.5">
      <span className="flex gap-0.5 items-end h-3">
        {[0, 1, 2].map((i) => (
          <span
            key={i}
            className="w-1 h-1 rounded-full bg-muted/60 animate-bounce"
            style={{ animationDelay: `${i * 0.15}s` }}
          />
        ))}
      </span>
      <span className="text-xs text-muted/70 italic">{label}</span>
    </div>
  );
}

function MessageRow({
  msg,
  channelId,
  grouped,
  isOwn,
  onOpenThread,
  onStartThread,
  onBookmark,
  onForward,
}: {
  msg: Message;
  channelId: string;
  grouped: boolean;
  isOwn: boolean;
  onOpenThread?: (threadId: string, title: string) => void;
  onStartThread?: (msgId: string, content: string) => void;
  onBookmark?: () => void;
  onForward?: () => void;
}) {
  const [pickerOpen, setPickerOpen] = useState(false);
  const pickerRef = useRef<HTMLDivElement>(null);

  // Close picker on outside click
  useEffect(() => {
    if (!pickerOpen) return;
    const handler = (e: MouseEvent) => {
      if (pickerRef.current && !pickerRef.current.contains(e.target as Node)) {
        setPickerOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [pickerOpen]);

  const handleReaction = async (emoji: string, currentlyMine: boolean) => {
    try {
      if (currentlyMine) {
        await invoke("remove_reaction", { channelId, messageId: msg.id, emoji });
      } else {
        await invoke("add_reaction", { channelId, messageId: msg.id, emoji });
      }
    } catch (e) {
      console.error("reaction error", e);
    }
    setPickerOpen(false);
  };
  return (
    <div
      className={clsx(
        "nexus-msg flex gap-3 px-1 py-0.5 rounded group hover:bg-bg-700/50 transition-colors",
        grouped ? "mt-0" : "mt-3"
      )}
    >
      {/* Avatar column */}
      <div className="w-9 shrink-0 mt-0.5">
        {!grouped ? (
          <div
            className={clsx(
              "w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold",
              isOwn ? "bg-accent-500 text-white" : "bg-bg-600 text-white"
            )}
          >
            {msg.authorUsername[0]?.toUpperCase()}
          </div>
        ) : (
          <span className="opacity-0 group-hover:opacity-100 text-[10px] text-muted/50 block text-right pr-1 leading-[2.25rem] transition-opacity select-none">
            {format(new Date(msg.createdAt), "HH:mm")}
          </span>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        {!grouped && (
          <div className="flex items-baseline gap-2 mb-0.5">
            <span className="text-sm font-semibold text-white">
              {msg.authorUsername}
            </span>
            <span className="text-xs text-muted">
              {formatDistanceToNow(new Date(msg.createdAt), { addSuffix: true })}
            </span>
          </div>
        )}
        <p className="text-sm text-gray-200 leading-relaxed break-words whitespace-pre-wrap">
          {msg.content}
        </p>
        {/* Thread badge */}
        {msg.threadId && (
          <button
            onClick={() => onOpenThread?.(msg.threadId!, `Thread`)}
            className="mt-1 inline-flex items-center gap-1.5 rounded border border-accent-500/30 bg-accent-500/10 px-2 py-0.5 text-xs text-accent-300 hover:bg-accent-500/20 transition-colors"
          >
            <span>🧵</span>
            <span className="font-medium">Open thread</span>
          </button>
        )}
        {/* Embeds */}
        {msg.embeds && msg.embeds.length > 0 && (
          <div className="mt-1.5 flex flex-col gap-1.5">
            {msg.embeds.map((embed, idx) => (
              <EmbedCard key={idx} embed={embed} />
            ))}
          </div>
        )}
        {msg.attachments && msg.attachments.length > 0 && (
          <div className="mt-1 flex flex-wrap gap-2">
            {msg.attachments.map((a) => (
              <a
                key={a.id}
                href={a.url}
                target="_blank"
                rel="noreferrer"
                className="text-accent-400 text-xs hover:underline"
              >
                📎 {a.filename}
              </a>
            ))}
          </div>
        )}
        {/* Reaction bar */}
        {((msg.reactions && msg.reactions.length > 0) || true) && (
          <div className="flex flex-wrap items-center gap-1 mt-1 relative">
            {(msg.reactions ?? []).map((r) => (
              <button
                key={r.emoji}
                onClick={() => handleReaction(r.emoji, r.me)}
                className={clsx(
                  "inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded-full border transition-colors",
                  r.me
                    ? "bg-accent-500/20 border-accent-500/50 text-accent-300 hover:bg-accent-500/30"
                    : "bg-bg-700/60 border-bg-600/50 text-muted hover:bg-bg-600/60 hover:text-fg"
                )}
                title={r.me ? `Remove ${r.emoji}` : `React with ${r.emoji}`}
              >
                <span>{r.emoji}</span>
                <span className="font-medium">{r.count}</span>
              </button>
            ))}
            {/* Add-reaction button */}
            <div ref={pickerRef} className="relative">
              <button
                onClick={() => setPickerOpen((v) => !v)}
                className="inline-flex items-center justify-center w-6 h-5 rounded-full border border-bg-600/50 bg-bg-700/40 text-muted hover:text-fg hover:bg-bg-600/60 transition-colors text-xs"
                title="Add reaction"
              >
                +
              </button>
              {pickerOpen && (
                <EmojiPicker onPick={(emoji) => handleReaction(emoji, (msg.reactions ?? []).find(r => r.emoji === emoji)?.me ?? false)} />
              )}
            </div>
            {/* Start/open thread button */}
            {!msg.threadId && onStartThread && (
              <button
                onClick={() => onStartThread(msg.id, msg.content ?? "")}
                className="inline-flex items-center justify-center h-5 px-1.5 rounded border border-bg-600/50 bg-bg-700/40 text-muted hover:text-fg hover:bg-bg-600/60 transition-colors text-xs opacity-0 group-hover:opacity-100"
                title="Start thread"
              >
                🧵
              </button>
            )}
            {/* Bookmark button */}
            {onBookmark && (
              <button
                onClick={onBookmark}
                className="inline-flex items-center justify-center h-5 px-1.5 rounded border border-bg-600/50 bg-bg-700/40 text-muted hover:text-fg hover:bg-bg-600/60 transition-colors text-xs opacity-0 group-hover:opacity-100"
                title="Bookmark message"
              >
                🔖
              </button>
            )}
            {/* Forward button — v0.14 */}
            {onForward && (
              <button
                onClick={onForward}
                className="inline-flex items-center justify-center h-5 px-1.5 rounded border border-bg-600/50 bg-bg-700/40 text-muted hover:text-fg hover:bg-bg-600/60 transition-colors text-xs opacity-0 group-hover:opacity-100"
                title="Forward message"
              >
                📤
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Embed card ────────────────────────────────────────────────────────────────
function EmbedCard({ embed }: { embed: Embed }) {
  const borderColor = embed.color
    ? `#${embed.color.toString(16).padStart(6, "0")}`
    : "#4f545c";
  return (
    <div
      className="max-w-lg rounded-r border-l-4 bg-bg-700/60 px-3 py-2.5 text-sm"
      style={{ borderLeftColor: borderColor }}
    >
      {embed.title && (
        embed.url ? (
          <a
            href={embed.url}
            target="_blank"
            rel="noreferrer"
            className="block mb-0.5 font-semibold text-accent-400 hover:underline"
          >
            {embed.title}
          </a>
        ) : (
          <p className="mb-0.5 font-semibold text-fg">{embed.title}</p>
        )
      )}
      {embed.description && (
        <p className="whitespace-pre-wrap leading-relaxed text-gray-400">
          {embed.description}
        </p>
      )}
      {(embed.thumbnail?.url ?? embed.image?.url) && (
        <img
          src={embed.thumbnail?.url ?? embed.image!.url}
          alt=""
          className="mt-2 max-h-40 rounded object-cover"
          loading="lazy"
        />
      )}
    </div>
  );
}

// ── Emoji picker ──────────────────────────────────────────────────────────────
const QUICK_EMOJIS = [
  "👍","👎","❤️","😂","😮","😢","🎉","🔥",
  "✅","❌","🤔","👀","💯","🙌","😍","😭",
  "🫡","💀","🚀","⭐",
];

function EmojiPicker({ onPick }: { onPick: (emoji: string) => void }) {
  return (
    <div className="absolute bottom-full left-0 z-50 mb-1 rounded-lg border border-bg-600/60 bg-bg-700 p-2 shadow-xl">
      <div className="grid grid-cols-5 gap-0.5">
        {QUICK_EMOJIS.map((e) => (
          <button
            key={e}
            onClick={() => onPick(e)}
            className="flex h-8 w-8 items-center justify-center rounded text-lg transition-colors hover:bg-bg-600/60"
          >
            {e}
          </button>
        ))}
      </div>
    </div>
  );
}

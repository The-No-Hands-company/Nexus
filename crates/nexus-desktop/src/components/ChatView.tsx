import { useEffect, useRef, useCallback } from "react";
import { useParams } from "react-router-dom";
import { useStore, Message } from "../store";
import MessageInput from "./MessageInput";
import { formatDistanceToNow } from "date-fns";
import clsx from "clsx";

export default function ChatView() {
  const { channelId } = useParams<{ channelId: string }>();
  const { messages, channels, session, loadMessages } = useStore();

  const msgs: Message[] = channelId ? (messages[channelId] ?? []) : [];
  const channel = channels.find((c) => c.id === channelId);
  const bottomRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (channelId) {
      loadMessages(channelId);
    }
  }, [channelId, loadMessages]);

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

  return (
    <div className="flex flex-col h-full bg-bg-800">
      {/* Channel header */}
      <div className="h-12 px-4 flex items-center gap-2 border-b border-bg-600 shrink-0 no-select">
        <span className="text-muted">#</span>
        <span className="font-semibold text-white text-sm">{channel?.name}</span>
        {channel?.isE2ee && (
          <span className="text-xs bg-green-900/40 text-green-400 px-1.5 py-0.5 rounded font-medium">
            E2EE
          </span>
        )}
      </div>

      {/* Messages */}
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-0.5"
      >
        {msgs.length === 0 && (
          <p className="text-muted text-sm text-center mt-16">
            No messages yet. Be the first!
          </p>
        )}
        {msgs.map((msg, i) => {
          const prevMsg = msgs[i - 1];
          const grouped =
            prevMsg?.authorId === msg.authorId &&
            new Date(msg.createdAt).getTime() -
              new Date(prevMsg.createdAt).getTime() <
              5 * 60 * 1000;
          return (
            <MessageRow
              key={msg.id}
              msg={msg}
              grouped={grouped}
              isOwn={msg.authorId === session?.userId}
            />
          );
        })}
        <div ref={bottomRef} />
      </div>

      {/* Input */}
      <MessageInput channelId={channelId} isE2ee={!!channel?.isE2ee} />
    </div>
  );
}

function MessageRow({
  msg,
  grouped,
  isOwn,
}: {
  msg: Message;
  grouped: boolean;
  isOwn: boolean;
}) {
  return (
    <div
      className={clsx(
        "flex gap-3 px-1 py-0.5 rounded group hover:bg-bg-700/50 transition-colors",
        grouped ? "mt-0" : "mt-3"
      )}
    >
      {/* Avatar column */}
      <div className="w-9 shrink-0 mt-0.5">
        {!grouped && (
          <div
            className={clsx(
              "w-9 h-9 rounded-full flex items-center justify-center text-sm font-bold",
              isOwn ? "bg-accent-500 text-white" : "bg-bg-600 text-white"
            )}
          >
            {msg.authorUsername[0]?.toUpperCase()}
          </div>
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
      </div>
    </div>
  );
}

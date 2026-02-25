import { useEffect, useRef } from "react";
import { useStore, Message } from "../store";
import MessageInput from "./MessageInput";
import { formatDistanceToNow } from "date-fns";
import clsx from "clsx";

interface Props {
  /** Channel ID of the thread (same as thread.id in the data model) */
  threadId: string;
  /** Human-readable title for the header */
  threadTitle: string;
  onClose: () => void;
}

export default function ThreadPanel({ threadId, threadTitle, onClose }: Props) {
  const { messages, loadMessages } = useStore();
  const msgs: Message[] = messages[threadId] ?? [];
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    loadMessages(threadId);
  }, [threadId, loadMessages]);

  // Auto-scroll when new messages arrive
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [msgs.length]);

  return (
    <aside className="flex h-full w-80 flex-shrink-0 flex-col border-l border-bg-600/40 bg-bg-800">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-bg-600/40 px-4 py-3">
        <div className="min-w-0 flex-1">
          <p className="text-[10px] font-semibold uppercase tracking-widest text-muted">
            Thread
          </p>
          <p className="truncate text-sm font-medium text-fg" title={threadTitle}>
            {threadTitle}
          </p>
        </div>
        <button
          onClick={onClose}
          className="ml-3 flex h-6 w-6 items-center justify-center rounded text-muted transition-colors hover:bg-bg-600/60 hover:text-fg"
          aria-label="Close thread"
        >
          ✕
        </button>
      </div>

      {/* Message list */}
      <div className="flex-1 overflow-y-auto px-3 py-3 space-y-2 scrollbar-thin scrollbar-thumb-bg-600/60">
        {msgs.length === 0 && (
          <p className="py-8 text-center text-xs text-muted">
            No messages yet — be the first to reply!
          </p>
        )}
        {msgs.map((msg, i) => {
          const prev = i > 0 ? msgs[i - 1] : null;
          const grouped =
            prev?.authorId === msg.authorId &&
            new Date(msg.createdAt).getTime() -
              new Date(prev!.createdAt).getTime() <
              5 * 60 * 1000;

          return (
            <div key={msg.id} className={clsx("flex gap-2", !grouped && "mt-3")}>
              {!grouped ? (
                <div className="mt-0.5 flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-accent-600/30 text-sm font-bold uppercase text-accent-300 select-none">
                  {msg.authorUsername.slice(0, 1)}
                </div>
              ) : (
                <div className="w-8 flex-shrink-0" />
              )}
              <div className="min-w-0 flex-1">
                {!grouped && (
                  <div className="mb-0.5 flex items-baseline gap-2">
                    <span className="text-xs font-semibold text-fg">
                      {msg.authorUsername}
                    </span>
                    <span className="text-[10px] text-muted">
                      {formatDistanceToNow(new Date(msg.createdAt), {
                        addSuffix: true,
                      })}
                    </span>
                  </div>
                )}
                <p className="break-words text-sm leading-relaxed text-fg/90 whitespace-pre-wrap">
                  {msg.content}
                </p>
              </div>
            </div>
          );
        })}
        <div ref={bottomRef} />
      </div>

      {/* Reply input */}
      <div className="border-t border-bg-600/40 px-2 py-2">
        <MessageInput channelId={threadId} isE2ee={false} />
      </div>
    </aside>
  );
}

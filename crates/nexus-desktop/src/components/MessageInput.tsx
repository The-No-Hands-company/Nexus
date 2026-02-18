import { useState, KeyboardEvent, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../store";
import clsx from "clsx";

interface Props {
  channelId: string;
  isE2ee: boolean;
}

export default function MessageInput({ channelId, isE2ee }: Props) {
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const { pttActive } = useStore();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const send = async () => {
    const content = text.trim();
    if (!content || sending) return;
    setSending(true);
    try {
      if (isE2ee) {
        // E2EE: encrypt per-recipient via Tauri command
        // For simplicity the command handles recipient key lookup server-side
        await invoke("send_encrypted_message", {
          channelId,
            ciphertextMap: {},  // populated by the Rust command after key lookup
          plaintextHint: content,
          attachmentIds: [],
        });
      } else {
        await invoke("send_message", {
          channelId,
          content,
          attachmentIds: [],
          replyToId: null,
        });
      }
      setText("");
      // Reset textarea height
      if (textareaRef.current) {
        textareaRef.current.style.height = "auto";
      }
    } catch (e) {
      console.error("send error", e);
    } finally {
      setSending(false);
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };

  const handleInput = () => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 200) + "px";
  };

  return (
    <div className="px-4 pb-4 pt-2 shrink-0">
      {/* PTT indicator */}
      {pttActive && (
        <div className="flex items-center gap-2 text-xs text-green-400 mb-1">
          <span className="w-2 h-2 rounded-full bg-green-400 animate-pulse inline-block" />
          Push-to-Talk active
        </div>
      )}

      <div
        className={clsx(
          "flex items-end gap-2 bg-bg-700 rounded-lg px-3 py-2",
          isE2ee && "ring-1 ring-green-700/50"
        )}
      >
        {isE2ee && (
          <span className="text-green-500 mb-1 shrink-0 text-sm" title="End-to-end encrypted">
            🔒
          </span>
        )}
        <textarea
          ref={textareaRef}
          rows={1}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          onInput={handleInput}
          placeholder={isE2ee ? "Message (encrypted)…" : "Message…"}
          className="flex-1 bg-transparent resize-none outline-none text-sm text-white placeholder-muted max-h-48 leading-relaxed"
          style={{ minHeight: "24px" }}
        />
        <button
          onClick={send}
          disabled={!text.trim() || sending}
          className="text-accent-400 hover:text-accent-300 disabled:text-muted transition-colors mb-0.5 shrink-0"
          title="Send (Enter)"
        >
          <SendIcon />
        </button>
      </div>
      <p className="text-xs text-muted mt-1 pl-1">
        Enter to send · Shift+Enter for newline
      </p>
    </div>
  );
}

function SendIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
      <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
    </svg>
  );
}

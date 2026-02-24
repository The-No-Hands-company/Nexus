import { useEffect, useRef, useState } from "react";
import { useStore, ServerMember } from "../store";
import clsx from "clsx";
import UserProfileCard from "./UserProfileCard";

interface Props {
  serverId: string;
}

const PRESENCE_DOT: Record<string, string> = {
  online: "bg-green-500",
  idle: "bg-yellow-400",
  do_not_disturb: "bg-red-500",
  invisible: "bg-bg-500",
  offline: "bg-bg-500",
};

function isOnline(m: ServerMember) {
  return m.presence === "online" || m.presence === "idle" || m.presence === "do_not_disturb";
}

export default function MemberList({ serverId }: Props) {
  const { members, loadMembers } = useStore();
  const serverMembers = members[serverId] ?? [];

  const [selected, setSelected] = useState<{ member: ServerMember; anchor: HTMLElement } | null>(null);

  useEffect(() => {
    if (serverId) loadMembers(serverId);
    // Refresh every 30 seconds to pick up presence changes
    const id = setInterval(() => loadMembers(serverId), 30_000);
    return () => clearInterval(id);
  }, [serverId, loadMembers]);

  const online = serverMembers.filter(isOnline);
  const offline = serverMembers.filter((m) => !isOnline(m));

  return (
    <div className="w-52 bg-bg-800 border-l border-bg-600/40 flex flex-col shrink-0 overflow-hidden">
      <div className="px-3 py-3 text-[11px] text-muted/70 font-semibold uppercase tracking-widest border-b border-bg-600/30 shrink-0 no-select">
        Members — {serverMembers.length}
      </div>

      <div className="flex-1 overflow-y-auto px-2 py-2 flex flex-col gap-px">
        {serverMembers.length === 0 && (
          <p className="text-muted text-xs text-center mt-8">Loading…</p>
        )}

        {online.length > 0 && (
          <>
            <p className="text-[10px] text-muted/60 uppercase tracking-widest font-medium px-2 py-1 select-none mt-1">
              Online — {online.length}
            </p>
            {online.map((m) => (
              <MemberRow
                key={m.userId}
                member={m}
                onSelect={(anchor) => setSelected({ member: m, anchor })}
                isSelected={selected?.member.userId === m.userId}
              />
            ))}
          </>
        )}

        {offline.length > 0 && (
          <>
            <p className="text-[10px] text-muted/60 uppercase tracking-widest font-medium px-2 py-1 select-none mt-2">
              Offline — {offline.length}
            </p>
            {offline.map((m) => (
              <MemberRow
                key={m.userId}
                member={m}
                onSelect={(anchor) => setSelected({ member: m, anchor })}
                isSelected={selected?.member.userId === m.userId}
              />
            ))}
          </>
        )}
      </div>

      {selected && (
        <UserProfileCard
          userId={selected.member.userId}
          username={selected.member.username}
          displayName={selected.member.displayName}
          avatar={selected.member.avatar}
          anchorEl={selected.anchor}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}

function MemberRow({
  member,
  onSelect,
  isSelected,
}: {
  member: ServerMember;
  onSelect: (anchor: HTMLElement) => void;
  isSelected: boolean;
}) {
  const btnRef = useRef<HTMLButtonElement>(null);
  const displayName = member.nickname ?? member.displayName ?? member.username;
  const initials = displayName.slice(0, 2).toUpperCase();
  const dotColor = PRESENCE_DOT[member.presence] ?? "bg-bg-500";
  const isOffline = member.presence === "offline" || member.presence === "invisible";

  return (
    <button
      ref={btnRef}
      onClick={() => btnRef.current && onSelect(btnRef.current)}
      className={clsx(
        "flex items-center gap-2 px-2 py-1.5 rounded w-full text-left transition-colors",
        isSelected
          ? "bg-bg-600 text-fg"
          : "hover:bg-bg-700/60 text-muted hover:text-fg"
      )}
    >
      {/* Avatar with presence dot */}
      <div className="relative shrink-0">
        {member.avatar ? (
          <img
            src={member.avatar}
            alt=""
            className={clsx("w-8 h-8 rounded-full object-cover", isOffline && "opacity-40")}
          />
        ) : (
          <div
            className={clsx(
              "w-8 h-8 rounded-full flex items-center justify-center text-[11px] font-bold",
              isOffline ? "bg-bg-600 text-muted/50" : "bg-accent-600 text-white"
            )}
          >
            {initials}
          </div>
        )}
        <span
          className={`absolute bottom-0 right-0 w-2.5 h-2.5 rounded-full border-2 border-bg-800 ${dotColor}`}
        />
      </div>

      {/* Name */}
      <div className="flex flex-col min-w-0">
        <span className={clsx("text-xs font-medium truncate", isOffline && "opacity-50")}>
          {displayName}
        </span>
        {member.muted && (
          <span className="text-[9px] text-red-400/70 leading-none">server muted</span>
        )}
      </div>
    </button>
  );
}

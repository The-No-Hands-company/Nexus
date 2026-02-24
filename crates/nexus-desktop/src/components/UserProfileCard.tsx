import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useStore } from "../store";
import { invoke } from "../invoke";

interface ProfileData {
  id: string;
  username: string;
  display_name?: string;
  avatar?: string;
  banner?: string;
  bio?: string;
  status?: string;
  presence: string;
  created_at: string;
  servers_in_common: { id: string; name: string; icon?: string; member_count: number }[];
  mutual_friends: { id: string; username: string; display_name?: string; avatar?: string }[];
}

interface Props {
  userId: string;
  /** Fallback display while loading */
  username: string;
  displayName?: string;
  avatar?: string;
  /** Position the card near this anchor element */
  anchorEl: HTMLElement | null;
  onClose: () => void;
}

const PRESENCE_COLOR: Record<string, string> = {
  online: "bg-green-500",
  idle: "bg-yellow-400",
  do_not_disturb: "bg-red-500",
  invisible: "bg-bg-500",
  offline: "bg-bg-500",
};

const PRESENCE_LABEL: Record<string, string> = {
  online: "Online",
  idle: "Idle",
  do_not_disturb: "Do Not Disturb",
  invisible: "Offline",
  offline: "Offline",
};

export default function UserProfileCard({ userId, username, displayName, avatar, anchorEl, onClose }: Props) {
  const navigate = useNavigate();
  const { relationships, loadRelationships, appendDmChannel, session } = useStore();
  const cardRef = useRef<HTMLDivElement>(null);
  const [profile, setProfile] = useState<ProfileData | null>(null);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [style, setStyle] = useState<React.CSSProperties>({ opacity: 0, pointerEvents: "none" });

  // Position the card to the right of (or left of if near edge) the anchor
  useEffect(() => {
    if (!anchorEl || !cardRef.current) return;
    const anchor = anchorEl.getBoundingClientRect();
    const card = cardRef.current.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    let left = anchor.right + 8;
    let top = anchor.top;

    // Flip left if overflowing right edge
    if (left + 300 > vw) left = anchor.left - 308;
    // Clamp vertically
    if (top + card.height > vh) top = vh - card.height - 8;
    if (top < 8) top = 8;

    setStyle({ position: "fixed", left, top, opacity: 1, pointerEvents: "auto", zIndex: 9999 });
  }, [anchorEl, loading]);

  // Load full profile
  useEffect(() => {
    setLoading(true);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    invoke<any>("get_user_profile", { userId })
      .then((data) => setProfile(data as ProfileData))
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [userId]);

  // Close on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [onClose]);

  const rel = relationships.find(
    (r) => r.user.id === userId || r.id === userId
  );
  const isFriend = rel?.status === "accepted";
  const isPending = rel?.status === "pending";

  const handleAddFriend = async () => {
    setSending(true);
    try {
      await invoke("send_friend_request", { username: profile?.username ?? username });
      await loadRelationships();
    } catch (e) {
      console.error("add friend error", e);
    } finally {
      setSending(false);
    }
  };

  const handleMessage = async () => {
    setSending(true);
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const dm = await invoke<any>("create_dm", { recipientId: userId });
      appendDmChannel(dm);
      navigate(`/channel/${dm.id}`);
      onClose();
    } catch (e) {
      console.error("create_dm error", e);
    } finally {
      setSending(false);
    }
  };

  const displayedName = profile?.display_name ?? displayName ?? username;
  const presence = profile?.presence ?? "offline";
  const presenceColor = PRESENCE_COLOR[presence] ?? "bg-bg-500";
  const initials = (displayedName).slice(0, 2).toUpperCase();

  return (
    <div
      ref={cardRef}
      style={style}
      className="w-72 bg-bg-700 rounded-xl shadow-2xl border border-bg-600/60 overflow-hidden transition-opacity duration-100"
    >
      {/* Banner / Avatar */}
      <div className="h-16 bg-gradient-to-br from-accent-700/60 to-accent-900/40 relative shrink-0">
        {profile?.banner && (
          <img src={profile.banner} alt="" className="w-full h-full object-cover" />
        )}
        <div className="absolute -bottom-6 left-4">
          <div className="relative w-16 h-16">
            {avatar ? (
              <img src={avatar} alt="" className="w-16 h-16 rounded-full border-4 border-bg-700 object-cover" />
            ) : (
              <div className="w-16 h-16 rounded-full border-4 border-bg-700 bg-accent-600 flex items-center justify-center text-white font-bold text-xl">
                {initials}
              </div>
            )}
            {/* Presence dot */}
            <span
              className={`absolute bottom-1 right-1 w-3.5 h-3.5 rounded-full border-2 border-bg-700 ${presenceColor}`}
              title={PRESENCE_LABEL[presence] ?? "Offline"}
            />
          </div>
        </div>
      </div>

      {/* Body */}
      <div className="pt-8 px-4 pb-4 flex flex-col gap-3">
        {/* Name + username */}
        <div>
          <p className="font-bold text-fg text-base leading-tight">{displayedName}</p>
          <p className="text-xs text-muted">@{profile?.username ?? username}</p>
          {profile?.status && (
            <p className="text-xs text-muted/80 mt-0.5 italic">{profile.status}</p>
          )}
        </div>

        {/* Bio */}
        {profile?.bio && (
          <p className="text-xs text-muted/80 leading-relaxed border-t border-bg-600/40 pt-2">
            {profile.bio}
          </p>
        )}

        {/* Servers in common */}
        {(profile?.servers_in_common?.length ?? 0) > 0 && (
          <div className="border-t border-bg-600/40 pt-2">
            <p className="text-[10px] uppercase tracking-widest text-muted/60 font-medium mb-1.5">
              {profile!.servers_in_common.length} Server{profile!.servers_in_common.length !== 1 ? "s" : ""} in Common
            </p>
            <div className="flex flex-col gap-1">
              {profile!.servers_in_common.slice(0, 3).map((s) => (
                <div key={s.id} className="flex items-center gap-2 text-xs text-muted hover:text-fg transition-colors">
                  <div className="w-5 h-5 rounded bg-accent-500/20 flex items-center justify-center text-[9px] font-bold text-accent-300 shrink-0">
                    {s.icon ? (
                      <img src={s.icon} alt="" className="w-5 h-5 rounded object-cover" />
                    ) : (
                      s.name.slice(0, 1).toUpperCase()
                    )}
                  </div>
                  <span className="truncate">{s.name}</span>
                  <span className="text-muted/50 shrink-0">{s.member_count}</span>
                </div>
              ))}
              {profile!.servers_in_common.length > 3 && (
                <p className="text-[10px] text-muted/50">+{profile!.servers_in_common.length - 3} more</p>
              )}
            </div>
          </div>
        )}

        {/* Mutual friends */}
        {(profile?.mutual_friends?.length ?? 0) > 0 && (
          <div className="border-t border-bg-600/40 pt-2">
            <p className="text-[10px] uppercase tracking-widest text-muted/60 font-medium mb-1.5">
              {profile!.mutual_friends.length} Mutual Friend{profile!.mutual_friends.length !== 1 ? "s" : ""}
            </p>
            <div className="flex gap-1 flex-wrap">
              {profile!.mutual_friends.slice(0, 5).map((f) => (
                <div
                  key={f.id}
                  title={f.display_name ?? f.username}
                  className="w-7 h-7 rounded-full bg-bg-600 flex items-center justify-center text-xs font-bold text-fg/70"
                >
                  {(f.display_name ?? f.username).slice(0, 2).toUpperCase()}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Action buttons */}
        {userId !== session?.userId && (
          <div className="flex gap-2 pt-1 border-t border-bg-600/40">
            <button
              onClick={handleMessage}
              disabled={sending}
              className="flex-1 text-xs bg-accent-600 hover:bg-accent-500 text-white rounded px-2 py-1.5 transition-colors disabled:opacity-50 font-medium"
            >
              Message
            </button>
            {!isFriend && !isPending && (
              <button
                onClick={handleAddFriend}
                disabled={sending}
                className="flex-1 text-xs bg-bg-600 hover:bg-bg-500 text-fg rounded px-2 py-1.5 transition-colors disabled:opacity-50 font-medium"
              >
                Add Friend
              </button>
            )}
            {isPending && rel?.direction === "outgoing" && (
              <button disabled className="flex-1 text-xs bg-bg-600/50 text-muted rounded px-2 py-1.5 cursor-default text-center font-medium">
                Pending
              </button>
            )}
            {isFriend && (
              <div className="flex-1 flex items-center justify-center gap-1 text-xs text-green-400 font-medium">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
                </svg>
                Friends
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

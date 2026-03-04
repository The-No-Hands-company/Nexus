import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useStore, InAppNotification } from "../store";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { formatDistanceToNow } from "date-fns";
import clsx from "clsx";

export default function NotificationTray() {
  const { inAppNotifications, unreadNotificationCount, markNotificationsRead, clearNotifications, setActiveChannel } =
    useStore();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const panelRef = useFocusTrap<HTMLDivElement>(open);

  // Mark all read when the tray is opened
  useEffect(() => {
    if (open && unreadNotificationCount > 0) {
      markNotificationsRead();
    }
  }, [open, unreadNotificationCount, markNotificationsRead]);

  // Close on Escape or outside click
  useEffect(() => {
    if (!open) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpen(false);
        buttonRef.current?.focus();
      }
    };
    const handleClick = (e: MouseEvent) => {
      if (
        panelRef.current &&
        !panelRef.current.contains(e.target as Node) &&
        !buttonRef.current?.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("keydown", handleKey);
    document.addEventListener("mousedown", handleClick);
    return () => {
      document.removeEventListener("keydown", handleKey);
      document.removeEventListener("mousedown", handleClick);
    };
  }, [open, panelRef]);

  const goToNotification = (notif: InAppNotification) => {
    if (!notif.channelId) {
      // Friend-request or other non-channel notification — open the Friends panel
      navigate("/home");
    } else {
      setActiveChannel(notif.channelId);
      navigate(`/channel/${notif.channelId}`);
    }
    setOpen(false);
  };

  const badge = unreadNotificationCount > 0 ? unreadNotificationCount : null;

  return (
    <div className="relative">
      {/* Bell button */}
      <button
        ref={buttonRef}
        onClick={() => setOpen((v) => !v)}
        title="Notifications"
        aria-label={
          badge
            ? `Notifications — ${badge} unread`
            : "Notifications"
        }
        aria-expanded={open}
        aria-haspopup="dialog"
        className={clsx(
          "relative p-1 rounded text-muted hover:text-fg hover:bg-bg-600/60 transition-colors",
          open && "text-fg bg-bg-600/60"
        )}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <path d="M12 22c1.1 0 2-.9 2-2h-4c0 1.1.9 2 2 2zm6-6v-5c0-3.07-1.63-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.64 5.36 6 7.92 6 11v5l-2 2v1h16v-1l-2-2z"/>
        </svg>
        {badge !== null && (
          <span
            aria-hidden="true"
            className="absolute -top-0.5 -right-0.5 min-w-[14px] h-3.5 rounded-full bg-red-500 text-white text-[9px] font-bold flex items-center justify-center px-0.5 leading-none"
          >
            {badge > 99 ? "99+" : badge}
          </span>
        )}
      </button>

      {/* Notification panel */}
      {open && (
        <div
          ref={panelRef}
          role="dialog"
          aria-modal="true"
          aria-label="Notifications"
          className="absolute right-0 top-full mt-1 w-80 rounded-xl bg-bg-700 border border-bg-600/50 shadow-2xl z-50 overflow-hidden"
        >
          {/* Header */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-bg-600/40">
            <h2 className="text-sm font-semibold text-fg">Notifications</h2>
            <div className="flex items-center gap-1">
              {inAppNotifications.length > 0 && (
                <button
                  onClick={clearNotifications}
                  className="text-[11px] text-muted hover:text-fg transition-colors px-2 py-0.5 rounded hover:bg-bg-600/50"
                  aria-label="Clear all notifications"
                >
                  Clear all
                </button>
              )}
              <button
                onClick={() => { setOpen(false); buttonRef.current?.focus(); }}
                className="text-muted hover:text-fg transition-colors p-0.5 rounded hover:bg-bg-600/50"
                aria-label="Close notifications"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                  <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                </svg>
              </button>
            </div>
          </div>

          {/* List */}
          <div
            role="list"
            aria-label="Recent notifications"
            className="max-h-96 overflow-y-auto"
          >
            {inAppNotifications.length === 0 ? (
              <p className="px-4 py-8 text-center text-sm text-muted/70">
                No notifications yet
              </p>
            ) : (
              inAppNotifications.map((notif) => (
                <button
                  key={notif.id}
                  role="listitem"
                  onClick={() => goToNotification(notif)}
                  aria-label={`${notif.authorUsername} mentioned you in ${notif.channelName ?? "a channel"}: ${notif.content}`}
                  className={clsx(
                    "w-full flex gap-3 px-4 py-3 text-left transition-colors hover:bg-bg-600/40 border-b border-bg-600/20 last:border-b-0",
                    !notif.read && "bg-accent-500/5"
                  )}
                >
                  {/* Author avatar */}
                  <div className="flex-shrink-0 w-8 h-8 rounded-full bg-bg-600 flex items-center justify-center text-xs font-bold text-fg/70 uppercase">
                    {notif.authorUsername[0]}
                  </div>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-baseline gap-1.5 mb-0.5">
                      <span className="text-xs font-semibold text-fg truncate">
                        {notif.authorUsername}
                      </span>
                      {notif.channelName && (
                        <span className="text-[10px] text-accent-400/80 truncate">
                          #{notif.channelName}
                        </span>
                      )}
                      {!notif.read && (
                        <span className="ml-auto shrink-0 w-1.5 h-1.5 rounded-full bg-accent-500" aria-hidden="true" />
                      )}
                    </div>
                    <p className="text-xs text-fg/70 leading-relaxed truncate">
                      {notif.content}
                    </p>
                    <p className="text-[10px] text-muted/60 mt-0.5">
                      {formatDistanceToNow(new Date(notif.createdAt), { addSuffix: true })}
                    </p>
                  </div>
                </button>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}

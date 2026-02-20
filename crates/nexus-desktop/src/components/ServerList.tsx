import { useState, type ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { useStore, Server } from "../store";
import clsx from "clsx";
import CreateServerModal from "./CreateServerModal";
import InviteModal from "./InviteModal";
import JoinServerModal from "./JoinServerModal";

export default function ServerList() {
  const { servers, activeServerId, setActiveServer, logout, session } =
    useStore();
  const navigate = useNavigate();
  const [showCreate, setShowCreate] = useState(false);
  const [showJoin, setShowJoin] = useState(false);
  const [inviteServer, setInviteServer] = useState<Server | null>(null);

  const handleSelectServer = (id: string) => {
    setActiveServer(id);
    navigate("/");
  };

  return (
    <>
      <div className="w-14 bg-bg-900 border-r border-bg-600/50 flex flex-col items-center py-2 overflow-y-auto shrink-0 no-select">

        {/* Nexus home */}
        <SpaceButton
          active={!activeServerId}
          onClick={() => { setActiveServer(null); navigate("/"); }}
          title="Home"
        >
          <span className="text-xs font-bold tracking-tight">NX</span>
        </SpaceButton>

        <div className="w-6 h-px bg-bg-600 my-2 shrink-0" />

        {/* Space icons */}
        {servers.map((srv) => (
          <div key={srv.id} className="group relative w-full flex items-center justify-center my-0.5">
            {/* Left active bar */}
            <div
              className={clsx(
                "absolute left-0 top-1/2 -translate-y-1/2 w-0.5 rounded-r transition-all duration-150",
                activeServerId === srv.id ? "h-6 bg-fg" : "h-3 bg-bg-500 opacity-0 group-hover:opacity-100"
              )}
            />
            <button
              onClick={() => handleSelectServer(srv.id)}
              title={srv.name}
              className={clsx(
                "w-9 h-9 rounded-lg flex items-center justify-center transition-colors duration-150 overflow-hidden",
                activeServerId === srv.id
                  ? "bg-accent-500 text-white"
                  : "bg-bg-700 text-fg hover:bg-accent-500/80 hover:text-white"
              )}
            >
              {srv.icon ? (
                <img src={srv.icon} alt={srv.name} className="w-full h-full object-cover" />
              ) : (
                <span className="text-xs font-bold">{srv.name.slice(0, 2).toUpperCase()}</span>
              )}
            </button>
            {/* Invite button — appears on hover in top-right corner */}
            <button
              onClick={(e) => { e.stopPropagation(); setInviteServer(srv); }}
              title={`Invite to ${srv.name}`}
              className="absolute top-0 right-1 w-4 h-4 rounded bg-bg-600 text-muted hover:bg-accent-500 hover:text-white opacity-0 group-hover:opacity-100 transition-all flex items-center justify-center z-10"
            >
              <svg width="9" height="9" viewBox="0 0 24 24" fill="currentColor">
                <path d="M17 7h-4v2h4c1.65 0 3 1.35 3 3s-1.35 3-3 3h-4v2h4c2.76 0 5-2.24 5-5s-2.24-5-5-5zm-6 8H7c-1.65 0-3-1.35-3-3s1.35-3 3-3h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-2zm1-4H8v2h8v-2z"/>
              </svg>
            </button>
          </div>
        ))}

        {/* Create server */}
        <SpaceButton
          active={false}
          onClick={() => setShowCreate(true)}
          title="Create Server"
          muted
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 13H13V19H11V13H5V11H11V5H13V11H19V13Z" />
          </svg>
        </SpaceButton>

        {/* Join server via invite */}
        <SpaceButton
          active={false}
          onClick={() => setShowJoin(true)}
          title="Join a Server"
          muted
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
            <path d="M17 7h-4v2h4c1.65 0 3 1.35 3 3s-1.35 3-3 3h-4v2h4c2.76 0 5-2.24 5-5s-2.24-5-5-5zm-6 8H7c-1.65 0-3-1.35-3-3s1.35-3 3-3h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-2zm1-4H8v2h8v-2z"/>
          </svg>
        </SpaceButton>

        {/* Push rest to bottom */}
        <div className="flex-1" />

        {/* Settings */}
        <button
          onClick={() => navigate("/settings")}
          className="w-8 h-8 rounded flex items-center justify-center text-muted hover:text-fg hover:bg-bg-700 transition-colors mb-1"
          title="Settings"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19.14 12.94c.04-.3.06-.61.06-.94s-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/>
          </svg>
        </button>

        {/* User avatar + logout */}
        <div className="flex flex-col items-center gap-1 pb-1">
          <div
            className="w-8 h-8 rounded-full bg-accent-500 flex items-center justify-center text-xs font-bold text-white cursor-default overflow-hidden"
            title={session?.username}
          >
            {session?.avatar ? (
              <img src={session.avatar} alt="" className="w-full h-full object-cover" />
            ) : (
              session?.username?.[0]?.toUpperCase()
            )}
          </div>
          <button
            onClick={logout}
            className="w-8 h-6 rounded flex items-center justify-center text-muted hover:text-red-400 transition-colors"
            title="Logout"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M17 7l-1.41 1.41L18.17 11H8v2h10.17l-2.58 2.58L17 17l5-5-5-5zM4 5h8V3H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h8v-2H4V5z" />
            </svg>
          </button>
        </div>
      </div>

      {showCreate && (
        <CreateServerModal onClose={() => setShowCreate(false)} />
      )}
      {showJoin && (
        <JoinServerModal onClose={() => setShowJoin(false)} />
      )}
      {inviteServer && (
        <InviteModal
          serverId={inviteServer.id}
          serverName={inviteServer.name}
          onClose={() => setInviteServer(null)}
        />
      )}
    </>
  );
}

// ── SpaceButton ───────────────────────────────────────────────────────────────
function SpaceButton({
  active,
  onClick,
  title,
  muted = false,
  children,
}: {
  active: boolean;
  onClick: () => void;
  title: string;
  muted?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="group relative w-full flex items-center justify-center my-0.5">
      {/* Left-edge active bar (VS Code style) */}
      <div
        className={clsx(
          "absolute left-0 top-1/2 -translate-y-1/2 w-0.5 rounded-r transition-all duration-150",
          active ? "h-6 bg-fg" : "h-3 bg-bg-500 opacity-0 group-hover:opacity-100"
        )}
      />
      <button
        onClick={onClick}
        title={title}
        className={clsx(
          "w-9 h-9 rounded-lg flex items-center justify-center transition-colors duration-150 overflow-hidden",
          active
            ? "bg-accent-500 text-white"
            : muted
              ? "bg-bg-700 text-muted hover:bg-bg-600 hover:text-fg"
              : "bg-bg-700 text-fg hover:bg-accent-500/80 hover:text-white"
        )}
      >
        {children}
      </button>
    </div>
  );
}

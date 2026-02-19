import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useStore } from "../store";
import clsx from "clsx";
import CreateServerModal from "./CreateServerModal";

export default function ServerList() {
  const { servers, activeServerId, setActiveServer, logout, session } =
    useStore();
  const navigate = useNavigate();
  const [showCreate, setShowCreate] = useState(false);

  const handleSelectServer = (id: string) => {
    setActiveServer(id);
    navigate("/");
  };

  return (
    <>
      <div className="w-[72px] bg-bg-900 flex flex-col items-center py-3 gap-2 overflow-y-auto shrink-0 no-select">
      {/* Nexus home button */}
      <button
        onClick={() => { setActiveServer(null); navigate("/"); }}
        className={clsx(
          "server-icon bg-bg-700 text-accent-400",
          !activeServerId && "ring-2 ring-accent-500"
        )}
        title="Home"
      >
        N
      </button>

      <div className="w-8 h-px bg-bg-600 my-1" />

      {/* Server icons */}
      {servers.map((srv) => (
        <button
          key={srv.id}
          onClick={() => handleSelectServer(srv.id)}
          className={clsx(
            "server-icon text-white",
            activeServerId === srv.id
              ? "ring-2 ring-accent-500 bg-accent-500"
              : "bg-bg-700 hover:bg-accent-500"
          )}
          title={srv.name}
        >
          {srv.icon ? (
            <img
              src={srv.icon}
              alt={srv.name}
              className="w-12 h-12 rounded-inherit object-cover"
            />
          ) : (
            srv.name.slice(0, 2).toUpperCase()
          )}
        </button>
      ))}

      {/* Add server button */}
      <button
        onClick={() => setShowCreate(true)}
        className="server-icon bg-bg-700 text-green-400 hover:bg-green-500 hover:text-white transition-colors"
        title="Create a Server"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M19 13H13V19H11V13H5V11H11V5H13V11H19V13Z" />
        </svg>
      </button>

      {/* Spacer push logout to bottom */}
      <div className="flex-1" />

      {/* Settings */}
      <button
        onClick={() => navigate("/settings")}
        className="text-muted hover:text-white transition-colors mb-1"
        title="Settings"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M19.14 12.94c.04-.3.06-.61.06-.94s-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/>
        </svg>
      </button>

      {/* User info + logout */}
      <div className="flex flex-col items-center gap-1">
        <div
          className="w-8 h-8 rounded-full bg-accent-500 flex items-center justify-center text-sm font-bold text-white cursor-default"
          title={session?.username}
        >
          {session?.username?.[0]?.toUpperCase()}
        </div>
        <button
          onClick={logout}
          className="text-muted hover:text-red-400 transition-colors text-xs"
          title="Logout"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
            <path d="M17 7l-1.41 1.41L18.17 11H8v2h10.17l-2.58 2.58L17 17l5-5-5-5zM4 5h8V3H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h8v-2H4V5z" />
          </svg>
        </button>
      </div>
    </div>

      {showCreate && (
        <CreateServerModal onClose={() => setShowCreate(false)} />
      )}
    </>
  );
}

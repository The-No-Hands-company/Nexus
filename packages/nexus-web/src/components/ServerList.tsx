import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useStore } from "../store";
import clsx from "clsx";

export default function ServerList() {
  const { servers, activeServerId, setActiveServer, loadChannels, session, setSession, loadDms } = useStore();
  const navigate = useNavigate();
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [homeMode, setHomeMode] = useState(false);

  const handleServer = (id: string) => {
    setHomeMode(false);
    setActiveServer(id);
    loadChannels(id);
    navigate("/");
  };

  const handleHome = () => {
    setHomeMode(true);
    setActiveServer(null);
    loadDms();
    navigate("/");
  };

  const handleCreate = async () => {
    if (!newName.trim()) return;
    try {
      const srv = await useStore.getState().createServer(newName.trim());
      setCreating(false);
      setNewName("");
      handleServer(srv.id);
    } catch (e) { console.error(e); }
  };

  return (
    <nav
      aria-label="Servers"
      className="w-14 bg-bg-900 border-r border-bg-600/40 flex flex-col items-center py-2 shrink-0 overflow-y-auto gap-1"
    >
      {/* Home */}
      <SpaceBtn
        active={homeMode}
        title="Home / DMs"
        onClick={handleHome}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/>
        </svg>
      </SpaceBtn>

      <div className="w-6 h-px bg-bg-600 my-1 shrink-0" />

      {servers.map((srv) => (
        <SpaceBtn
          key={srv.id}
          active={activeServerId === srv.id}
          title={srv.name}
          onClick={() => handleServer(srv.id)}
        >
          {srv.icon ? (
            <img src={srv.icon} alt="" className="w-full h-full object-cover rounded-xl" />
          ) : (
            <span className="text-xs font-bold leading-none">
              {srv.name.slice(0, 2).toUpperCase()}
            </span>
          )}
        </SpaceBtn>
      ))}

      <div className="flex-1" />

      {/* Create server */}
      {creating ? (
        <div className="w-10 flex flex-col gap-1">
          <input
            autoFocus
            className="nx-input text-[11px] px-1 py-1 text-center"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCreate();
              if (e.key === "Escape") { setCreating(false); setNewName(""); }
            }}
            maxLength={50}
            placeholder="Name"
          />
        </div>
      ) : (
        <SpaceBtn title="Create a server" onClick={() => setCreating(true)}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
          </svg>
        </SpaceBtn>
      )}

      {/* Logout */}
      <button
        onClick={() => setSession(null)}
        title={`Logout (${session?.username})`}
        className="w-8 h-8 rounded-full bg-accent-500/20 flex items-center justify-center text-accent-300 text-xs font-bold hover:bg-red-900/40 hover:text-red-400 transition-colors mt-1"
      >
        {session?.username?.[0]?.toUpperCase() ?? "?"}
      </button>
    </nav>
  );
}

function SpaceBtn({
  children, active, title, onClick,
}: {
  children: React.ReactNode;
  active?: boolean;
  title: string;
  onClick: () => void;
}) {
  return (
    <div className="relative w-full flex items-center justify-center">
      {/* Active indicator */}
      <div className={clsx(
        "absolute left-0 top-1/2 -translate-y-1/2 w-0.5 rounded-r transition-all",
        active ? "h-6 bg-fg" : "h-2 bg-muted opacity-0 group-hover:opacity-100"
      )} />
      <button
        title={title}
        onClick={onClick}
        className={clsx(
          "w-10 h-10 rounded-xl flex items-center justify-center text-muted transition-all",
          "hover:rounded-lg hover:bg-accent-500/20 hover:text-accent-300",
          active && "!rounded-lg bg-accent-500/20 text-accent-300"
        )}
      >
        {children}
      </button>
    </div>
  );
}

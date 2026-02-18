import { useNavigate } from "react-router-dom";
import { useStore } from "../store";
import clsx from "clsx";

export default function ServerList() {
  const { servers, activeServerId, setActiveServer, logout, session } =
    useStore();
  const navigate = useNavigate();

  const handleSelectServer = (id: string) => {
    setActiveServer(id);
    navigate("/");
  };

  return (
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

      {/* Spacer push logout to bottom */}
      <div className="flex-1" />

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
  );
}

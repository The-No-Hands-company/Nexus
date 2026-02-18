import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../store";

export default function UpdateBanner() {
  const { updateAvailable, setUpdateAvailable } = useStore();

  if (!updateAvailable) return null;

  const handleInstall = async () => {
    try {
      await invoke("install_update");
    } catch (e) {
      console.error("install_update error", e);
    }
  };

  return (
    <div className="flex items-center justify-between px-4 py-2 bg-accent-500/20 border-b border-accent-500/30 text-sm shrink-0 no-select">
      <p className="text-white">
        <span className="font-semibold">Nexus {updateAvailable.version}</span> is available
        {updateAvailable.body ? ` — ${updateAvailable.body}` : "."}
      </p>
      <div className="flex gap-2">
        <button
          onClick={handleInstall}
          className="btn-primary text-xs px-3 py-1"
        >
          Install &amp; Restart
        </button>
        <button
          onClick={() => setUpdateAvailable(null)}
          className="btn-ghost text-xs"
        >
          Later
        </button>
      </div>
    </div>
  );
}

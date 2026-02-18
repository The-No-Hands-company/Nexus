/**
 * Listen for PTT start/stop events emitted by the Tauri hotkeys module.
 */
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "../store";

export function usePtt() {
  const { setPttActive } = useStore();

  useEffect(() => {
    const unlistenStart = listen("ptt-start", () => setPttActive(true));
    const unlistenStop = listen("ptt-stop", () => setPttActive(false));

    return () => {
      unlistenStart.then((fn) => fn());
      unlistenStop.then((fn) => fn());
    };
  }, [setPttActive]);
}

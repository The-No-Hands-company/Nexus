import { useEffect, useRef } from "react";
import type { PluginManifest, PluginMessage } from "./types";

interface Props {
  manifest: PluginManifest;
}

/**
 * Renders one plugin inside a sandboxed <iframe>.
 *
 * The sandbox only allows `allow-scripts` — no same-origin, no popups,
 * no top navigation. All plugin↔host communication goes through postMessage.
 *
 * The plugin (running inside the iframe) can call:
 *   window.parent.postMessage({ type: "nexus:ready", pluginId: "..." }, "*")
 *
 * The host can push events to the plugin via:
 *   iframe.contentWindow.postMessage({ type: "nexus:dispatch", event: "...", data: {} }, "*")
 */
export default function PluginSandbox({ manifest }: Props) {
  const iframeRef = useRef<HTMLIFrameElement>(null);

  // Listen for messages from this plugin's iframe
  useEffect(() => {
    const handler = (ev: MessageEvent) => {
      // Ignore messages from other origins / that are not from this plugin's iframe
      if (!iframeRef.current?.contentWindow) return;
      if (ev.source !== iframeRef.current.contentWindow) return;

      const msg = ev.data as PluginMessage;
      if (!msg?.type?.startsWith("nexus:")) return;

      switch (msg.type) {
        case "nexus:ready":
          console.log(`[plugin:${manifest.id}] ready`);
          break;
        case "nexus:log":
          console.log(`[plugin:${manifest.id}]`, msg.payload);
          break;
        case "nexus:error":
          console.error(`[plugin:${manifest.id}] error`, msg.payload);
          break;
        default:
          break;
      }
    };

    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [manifest.id]);

  return (
    <iframe
      ref={iframeRef}
      src={manifest.url}
      sandbox="allow-scripts"
      title={`plugin-${manifest.id}`}
      style={{ display: "none" }}
      aria-hidden
    />
  );
}

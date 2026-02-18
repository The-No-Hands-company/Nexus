export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  /** URL of the plugin's entry HTML (loaded in a sandboxed iframe). */
  url: string;
  iconUrl?: string;
}

/** Message formats sent PLUGIN → HOST via postMessage. */
export interface PluginMessage {
  type: "nexus:ready" | "nexus:event" | "nexus:log" | "nexus:error";
  pluginId: string;
  payload?: unknown;
}

/** Message formats sent HOST → PLUGIN via contentWindow.postMessage. */
export interface HostMessage {
  type: "nexus:dispatch";
  event: string;
  data: unknown;
}

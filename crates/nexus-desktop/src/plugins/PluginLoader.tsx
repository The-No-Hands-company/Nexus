import { useStore } from "../store";
import PluginSandbox from "./PluginSandbox";

/**
 * Mounts background sandboxed iframes for every enabled plugin.
 * Place this once near the root of the app tree.
 */
export default function PluginLoader() {
  const plugins = useStore((s) => s.plugins);
  const enabledPlugins = useStore((s) => s.enabledPlugins);

  const enabled = plugins.filter((p) => enabledPlugins.includes(p.id));

  return (
    <>
      {enabled.map((manifest) => (
        <PluginSandbox key={manifest.id} manifest={manifest} />
      ))}
    </>
  );
}

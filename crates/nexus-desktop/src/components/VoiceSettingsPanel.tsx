import { useEffect } from "react";
import { useStore, VoiceSettings } from "../store";
import { invoke } from "../invoke";

/**
 * Voice settings panel for configuring spatial audio, noise gate,
 * echo cancellation, and auto-gain.
 */
export default function VoiceSettingsPanel({ onClose }: { onClose: () => void }) {
  const { voiceSettings, setVoiceSettings, loadVoiceSettings } = useStore();

  useEffect(() => {
    loadVoiceSettings();
  }, []);

  const update = async (patch: Partial<VoiceSettings>) => {
    try {
      const updated = await invoke<VoiceSettings>("update_voice_settings", {
        spatialAudio: patch.spatialAudio,
        noiseGateEnabled: patch.noiseGateEnabled,
        noiseGateThreshold: patch.noiseGateThreshold,
        echoCancelLevel: patch.echoCancelLevel,
        autoGain: patch.autoGain,
      });
      setVoiceSettings(updated);
    } catch (e) {
      console.error("update voice settings error", e);
    }
  };

  if (!voiceSettings) {
    return (
      <div className="p-4 text-muted text-center">Loading voice settings…</div>
    );
  }

  return (
    <div className="flex flex-col bg-surface rounded-lg border border-border max-w-md">
      <div className="flex items-center justify-between p-3 border-b border-border">
        <h3 className="font-semibold">Voice Settings</h3>
        <button onClick={onClose} className="text-muted hover:text-foreground">
          ✕
        </button>
      </div>

      <div className="p-4 space-y-4">
        {/* Spatial Audio */}
        <label className="flex items-center justify-between">
          <span className="text-sm">Spatial Audio</span>
          <input
            type="checkbox"
            checked={voiceSettings.spatialAudio}
            onChange={(e) => update({ spatialAudio: e.target.checked })}
            className="accent-accent"
          />
        </label>

        {/* Noise Gate */}
        <div>
          <label className="flex items-center justify-between mb-1">
            <span className="text-sm">Noise Gate</span>
            <input
              type="checkbox"
              checked={voiceSettings.noiseGateEnabled}
              onChange={(e) => update({ noiseGateEnabled: e.target.checked })}
              className="accent-accent"
            />
          </label>
          {voiceSettings.noiseGateEnabled && (
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted">Threshold</span>
              <input
                type="range"
                min={-100}
                max={0}
                step={1}
                value={voiceSettings.noiseGateThreshold}
                onChange={(e) =>
                  update({ noiseGateThreshold: Number(e.target.value) })
                }
                className="flex-1"
              />
              <span className="text-xs text-muted w-12 text-right">
                {voiceSettings.noiseGateThreshold} dB
              </span>
            </div>
          )}
        </div>

        {/* Echo Cancellation */}
        <div>
          <span className="text-sm block mb-1">Echo Cancellation</span>
          <div className="flex gap-1">
            {(["off", "low", "moderate", "aggressive"] as const).map((level) => (
              <button
                key={level}
                onClick={() => update({ echoCancelLevel: level })}
                className={`px-2 py-1 text-xs rounded ${
                  voiceSettings.echoCancelLevel === level
                    ? "bg-accent text-white"
                    : "bg-surface-secondary text-muted hover:bg-surface-tertiary"
                }`}
              >
                {level.charAt(0).toUpperCase() + level.slice(1)}
              </button>
            ))}
          </div>
        </div>

        {/* Auto Gain */}
        <label className="flex items-center justify-between">
          <span className="text-sm">Auto Gain Control</span>
          <input
            type="checkbox"
            checked={voiceSettings.autoGain}
            onChange={(e) => update({ autoGain: e.target.checked })}
            className="accent-accent"
          />
        </label>
      </div>
    </div>
  );
}

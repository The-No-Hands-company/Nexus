import { useState, useEffect } from "react";
import { useStore } from "../store";
import { invoke } from "../invoke";
import clsx from "clsx";
import {
  type FontFamily,
  type ColorBlindMode,
  getFontFamily,
  getReducedMotion,
  getRtl,
  getColorBlindMode,
  saveFontFamily,
  saveReducedMotion,
  saveRtl,
  saveColorBlindMode,
} from "../themes/appearance";

export default function AccessibilitySettingsPanel() {
  const { accessibilitySettings, loadAccessibilitySettings, setAccessibilitySettings } = useStore();
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  // Local appearance state
  const [fontFamily, setFontFamily] = useState<FontFamily>(() => getFontFamily());
  const [reducedMotion, setReducedMotion] = useState(() => getReducedMotion());
  const [rtl, setRtl] = useState(() => getRtl());
  const [colorBlindMode, setColorBlindMode] = useState<ColorBlindMode>(() => getColorBlindMode());

  // Server-side settings
  const [screenReader, setScreenReader] = useState(false);
  const [announceMessages, setAnnounceMessages] = useState(true);
  const [announceReactions, setAnnounceReactions] = useState(true);
  const [announceTyping, setAnnounceTyping] = useState(false);
  const [keyboardShortcuts, setKeyboardShortcuts] = useState(true);
  const [highContrast, setHighContrast] = useState(false);
  const [captionsEnabled, setCaptionsEnabled] = useState(false);
  const [captionFontSize, setCaptionFontSize] = useState("md");
  const [captionPosition, setCaptionPosition] = useState("bottom");
  const [ttsEnabled, setTtsEnabled] = useState(false);
  const [ttsRate, setTtsRate] = useState(1.0);
  const [ttsVoice, setTtsVoice] = useState<string | null>(null);
  const [preferredLanguage, setPreferredLanguage] = useState("en");
  const [autoTranslate, setAutoTranslate] = useState(false);

  useEffect(() => {
    loadAccessibilitySettings();
  }, [loadAccessibilitySettings]);

  useEffect(() => {
    if (!accessibilitySettings) return;
    setScreenReader(accessibilitySettings.screen_reader_mode ?? false);
    setAnnounceMessages(accessibilitySettings.announce_messages ?? true);
    setAnnounceReactions(accessibilitySettings.announce_reactions ?? true);
    setAnnounceTyping(accessibilitySettings.announce_typing ?? false);
    setKeyboardShortcuts(accessibilitySettings.keyboard_shortcuts ?? true);
    setHighContrast(accessibilitySettings.high_contrast_mode ?? false);
    setCaptionsEnabled(accessibilitySettings.captions_enabled ?? false);
    setCaptionFontSize(accessibilitySettings.caption_font_size ?? "md");
    setCaptionPosition(accessibilitySettings.caption_position ?? "bottom");
    setTtsEnabled(accessibilitySettings.tts_enabled ?? false);
    setTtsRate(accessibilitySettings.tts_rate ?? 1.0);
    setTtsVoice(accessibilitySettings.tts_voice ?? null);
    setPreferredLanguage(accessibilitySettings.preferred_language ?? "en");
    setAutoTranslate(accessibilitySettings.auto_translate ?? false);
  }, [accessibilitySettings]);

  async function saveSettings() {
    setSaving(true);
    setMsg(null);
    try {
      const updated = await invoke("update_accessibility_settings", {
        screen_reader_mode: screenReader,
        announce_messages: announceMessages,
        announce_reactions: announceReactions,
        announce_typing: announceTyping,
        keyboard_shortcuts: keyboardShortcuts,
        high_contrast_mode: highContrast,
        reduced_motion: reducedMotion,
        font_family: fontFamily,
        color_blind_mode: colorBlindMode === "none" ? null : colorBlindMode,
        preferred_language: preferredLanguage,
        auto_translate: autoTranslate,
        captions_enabled: captionsEnabled,
        caption_font_size: captionFontSize,
        caption_position: captionPosition,
        tts_enabled: ttsEnabled,
        tts_rate: ttsRate,
        tts_voice: ttsVoice,
      });
      setAccessibilitySettings(updated);
      setMsg("Settings saved.");
    } catch {
      setMsg("Failed to save settings.");
    } finally {
      setSaving(false);
    }
  }

  const Toggle = ({ label, checked, onChange, description }: {
    label: string; checked: boolean; onChange: (v: boolean) => void; description?: string;
  }) => (
    <label className="flex items-start gap-3 cursor-pointer group">
      <div className="relative flex-shrink-0 mt-0.5">
        <input
          type="checkbox"
          className="sr-only peer"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
        />
        <div className={clsx(
          "w-9 h-5 rounded-full transition-colors",
          checked ? "bg-accent-500" : "bg-bg-500"
        )} />
        <div className={clsx(
          "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform",
          checked && "translate-x-4"
        )} />
      </div>
      <div>
        <span className="text-sm text-fg group-hover:text-accent-300 transition-colors">{label}</span>
        {description && <p className="text-xs text-muted mt-0.5">{description}</p>}
      </div>
    </label>
  );

  return (
    <div className="space-y-8">
      {/* ── Screen Reader ────────────────────────────── */}
      <div>
        <h3 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Screen Reader</h3>
        <div className="space-y-3 max-w-lg">
          <Toggle label="Screen reader mode" checked={screenReader} onChange={setScreenReader}
            description="Optimise UI for assistive technology. Adds extra ARIA labels." />
          <Toggle label="Announce new messages" checked={announceMessages} onChange={setAnnounceMessages}
            description="Screen reader announces incoming messages via live region." />
          <Toggle label="Announce reactions" checked={announceReactions} onChange={setAnnounceReactions} />
          <Toggle label="Announce typing indicators" checked={announceTyping} onChange={setAnnounceTyping} />
          <Toggle label="Keyboard shortcuts" checked={keyboardShortcuts} onChange={setKeyboardShortcuts}
            description="Enable global keyboard shortcuts for navigation." />
        </div>
      </div>

      {/* ── Visual ───────────────────────────────────── */}
      <div>
        <h3 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Visual</h3>
        <div className="space-y-4 max-w-lg">
          <Toggle label="High contrast mode" checked={highContrast} onChange={setHighContrast}
            description="Use maximum-contrast color themes (WCAG AAA)." />
          <Toggle label="Reduced motion" checked={reducedMotion} onChange={(v) => {
            setReducedMotion(v); saveReducedMotion(v);
          }} description="Disable animations and transitions." />
          <Toggle label="RTL layout" checked={rtl} onChange={(v) => {
            setRtl(v); saveRtl(v);
          }} description="Right-to-left interface direction." />

          {/* Font Family */}
          <div>
            <p className="text-xs font-semibold text-muted uppercase tracking-wider mb-2">Font Family</p>
            <div className="flex gap-1.5">
              {([
                { id: "system" as const, label: "Default" },
                { id: "monospace" as const, label: "Mono" },
                { id: "dyslexic" as const, label: "Dyslexic" },
                { id: "serif" as const, label: "Serif" },
              ]).map(({ id, label }) => (
                <button key={id} aria-pressed={fontFamily === id}
                  onClick={() => { setFontFamily(id); saveFontFamily(id); }}
                  className={clsx(
                    "flex-1 rounded-lg border py-2 text-sm font-medium transition-colors",
                    fontFamily === id
                      ? "border-accent-600 bg-accent-600/15 text-accent-300"
                      : "border-bg-600/50 bg-bg-800 text-muted hover:text-fg"
                  )}>
                  {label}
                </button>
              ))}
            </div>
          </div>

          {/* Color-blind mode */}
          <div>
            <p className="text-xs font-semibold text-muted uppercase tracking-wider mb-2">Color-Blind Mode</p>
            <div className="flex gap-1.5 flex-wrap">
              {([
                { id: "none" as const, label: "None" },
                { id: "protanopia" as const, label: "Protanopia" },
                { id: "deuteranopia" as const, label: "Deuteranopia" },
                { id: "tritanopia" as const, label: "Tritanopia" },
              ]).map(({ id, label }) => (
                <button key={id} aria-pressed={colorBlindMode === id}
                  onClick={() => { setColorBlindMode(id); saveColorBlindMode(id); }}
                  className={clsx(
                    "rounded-lg border px-3 py-2 text-sm font-medium transition-colors",
                    colorBlindMode === id
                      ? "border-accent-600 bg-accent-600/15 text-accent-300"
                      : "border-bg-600/50 bg-bg-800 text-muted hover:text-fg"
                  )}>
                  {label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* ── Language & Translation ────────────────────── */}
      <div>
        <h3 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Language</h3>
        <div className="space-y-3 max-w-lg">
          <div>
            <label className="block text-xs text-muted mb-1">Preferred Language</label>
            <input type="text" value={preferredLanguage}
              onChange={(e) => setPreferredLanguage(e.target.value.slice(0, 5))}
              placeholder="en" maxLength={5}
              className="input max-w-[6rem]" />
          </div>
          <Toggle label="Auto-translate messages" checked={autoTranslate} onChange={setAutoTranslate}
            description="Automatically translate messages not in your preferred language." />
        </div>
      </div>

      {/* ── Captions & TTS ───────────────────────────── */}
      <div>
        <h3 className="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Voice Accessibility</h3>
        <div className="space-y-4 max-w-lg">
          <Toggle label="Live captions" checked={captionsEnabled} onChange={setCaptionsEnabled}
            description="Show real-time captions during voice calls." />

          {captionsEnabled && (
            <div className="pl-12 space-y-3">
              <div>
                <p className="text-xs text-muted mb-1">Caption Size</p>
                <div className="flex gap-1.5">
                  {(["sm", "md", "lg", "xl"] as const).map((s) => (
                    <button key={s} aria-pressed={captionFontSize === s}
                      onClick={() => setCaptionFontSize(s)}
                      className={clsx(
                        "flex-1 rounded border py-1.5 text-xs font-medium transition-colors uppercase",
                        captionFontSize === s
                          ? "border-accent-600 bg-accent-600/15 text-accent-300"
                          : "border-bg-600/50 bg-bg-800 text-muted hover:text-fg"
                      )}>
                      {s}
                    </button>
                  ))}
                </div>
              </div>
              <div>
                <p className="text-xs text-muted mb-1">Position</p>
                <div className="flex gap-1.5">
                  {(["top", "bottom"] as const).map((p) => (
                    <button key={p} aria-pressed={captionPosition === p}
                      onClick={() => setCaptionPosition(p)}
                      className={clsx(
                        "flex-1 rounded border py-1.5 text-xs font-medium transition-colors capitalize",
                        captionPosition === p
                          ? "border-accent-600 bg-accent-600/15 text-accent-300"
                          : "border-bg-600/50 bg-bg-800 text-muted hover:text-fg"
                      )}>
                      {p}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}

          <Toggle label="Text-to-speech" checked={ttsEnabled} onChange={setTtsEnabled}
            description="Read messages aloud using text-to-speech." />

          {ttsEnabled && (
            <div className="pl-12 space-y-3">
              <div>
                <label className="block text-xs text-muted mb-1">Speech Rate ({ttsRate.toFixed(1)}x)</label>
                <input type="range" min="0.5" max="2.0" step="0.1"
                  value={ttsRate} onChange={(e) => setTtsRate(parseFloat(e.target.value))}
                  className="w-full accent-accent-500" />
              </div>
              <div>
                <label className="block text-xs text-muted mb-1">Voice (optional)</label>
                <input type="text" value={ttsVoice ?? ""}
                  onChange={(e) => setTtsVoice(e.target.value || null)}
                  placeholder="Default voice"
                  className="input max-w-[12rem]" />
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ── Save Button ──────────────────────────────── */}
      <div className="flex items-center gap-3">
        <button onClick={saveSettings} disabled={saving}
          className="btn-primary disabled:opacity-50">
          {saving ? "Saving…" : "Save Accessibility Settings"}
        </button>
        {msg && <span className="text-xs text-muted">{msg}</span>}
      </div>
    </div>
  );
}

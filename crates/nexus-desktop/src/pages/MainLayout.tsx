import { useEffect, useRef } from "react";
import { Routes, Route } from "react-router-dom";
import { useStore } from "../store";
import { useGateway } from "../hooks/useGateway";
import { usePtt } from "../hooks/usePtt";
import { invoke } from "../invoke";
import ServerList from "../components/ServerList";
import ChannelList from "../components/ChannelList";
import ChatView from "../components/ChatView";
import VoiceChannel from "../components/VoiceChannel";
import SettingsPage from "./Settings";
import FriendsPanel from "../components/FriendsPanel";
import AdminAnalyticsPanel from "../components/AdminAnalyticsPanel";
import OnboardingWizard from "../components/OnboardingWizard";
import MediaGalleryPanel from "../components/MediaGalleryPanel";
import CalendarPanel from "../components/CalendarPanel";
import StoryViewer from "../components/StoryViewer";
import ImportWizard from "../components/ImportWizard";
import VoiceSettingsPanel from "../components/VoiceSettingsPanel";

export default function MainLayout() {
  const {
    loadServers,
    activeServerId,
    loadChannels,
    loadExperienceProfile,
    experienceProfile,
    // overlay panel state
    mediaGalleryOpen, setMediaGalleryOpen,
    calendarOpen, setCalendarOpen,
    storyViewerOpen, setStoryViewerOpen,
    storyFeed, loadStoryFeed,
    importWizardOpen, setImportWizardOpen,
    voiceSettingsOpen, setVoiceSettingsOpen,
    // onboarding
    onboardingProgress, loadOnboardingProgress,
  } = useStore();

  // Open gateway WebSocket
  useGateway();
  // Listen for PTT events from Tauri
  usePtt();

  // Proactively refresh the access token every 10 minutes (TTL is 15 min).
  const refreshTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  useEffect(() => {
    const REFRESH_INTERVAL_MS = 10 * 60 * 1000;
    refreshTimerRef.current = setInterval(() => {
      invoke("refresh_token").catch(() => {});
    }, REFRESH_INTERVAL_MS);
    return () => {
      if (refreshTimerRef.current) clearInterval(refreshTimerRef.current);
    };
  }, []);

  useEffect(() => {
    loadServers();
    loadExperienceProfile();
    loadOnboardingProgress();
    loadStoryFeed();
  }, [loadServers, loadExperienceProfile, loadOnboardingProgress, loadStoryFeed]);

  useEffect(() => {
    if (activeServerId) {
      loadChannels(activeServerId);
    }
  }, [activeServerId, loadChannels]);

  // Show onboarding wizard for brand-new users who haven't completed it
  const showOnboarding =
    onboardingProgress !== null &&
    !onboardingProgress.dismissed &&
    Object.values(onboardingProgress.steps ?? {}).some((v) => !v);

  return (
    <div className="flex h-full overflow-hidden">
      {/* Column 1: Server list (icon rail) */}
      {!experienceProfile?.messagingMode && <ServerList />}

      {/* Column 2: Channel list */}
      <ChannelList />

      {/* Column 3: Main content */}
      <div className="flex flex-col flex-1 overflow-hidden">
        <Routes>
          <Route path="/" element={<div className="flex-1 flex items-center justify-center text-muted text-sm">Select a channel</div>} />
          <Route path="/home" element={<FriendsPanel />} />
          <Route path="/channel/:channelId" element={<ChatView />} />
          <Route path="/voice/:channelId" element={<VoiceChannel />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route
            path="/admin"
            element={
              activeServerId ? (
                <div className="flex-1 overflow-y-auto p-6">
                  <h1 className="text-lg font-semibold mb-6">Server Analytics</h1>
                  <AdminAnalyticsPanel serverId={activeServerId} />
                </div>
              ) : (
                <div className="flex-1 flex items-center justify-center text-muted text-sm">
                  Select a server to view analytics
                </div>
              )
            }
          />
          <Route
            path="/onboarding"
            element={
              <div className="flex-1 flex items-center justify-center p-8">
                <OnboardingWizard />
              </div>
            }
          />
        </Routes>
      </div>

      {/* ── Overlay panels ──────────────────────────────────────────────── */}

      {/* Media Gallery */}
      {mediaGalleryOpen && (
        <OverlayPanel onClose={() => setMediaGalleryOpen(false)} title="Media Gallery">
          <MediaGalleryPanel
            serverId={activeServerId ?? undefined}
            onClose={() => setMediaGalleryOpen(false)}
          />
        </OverlayPanel>
      )}

      {/* Calendar */}
      {calendarOpen && activeServerId && (
        <OverlayPanel onClose={() => setCalendarOpen(false)} title="Calendar">
          <CalendarPanel
            serverId={activeServerId}
            onClose={() => setCalendarOpen(false)}
          />
        </OverlayPanel>
      )}

      {/* Story viewer */}
      {storyViewerOpen && storyFeed.length > 0 && (
        <StoryViewer
          stories={storyFeed}
          startIndex={0}
          onClose={() => setStoryViewerOpen(false)}
        />
      )}

      {/* Import wizard */}
      {importWizardOpen && activeServerId && (
        <OverlayPanel onClose={() => setImportWizardOpen(false)} title="Import Data">
          <ImportWizard serverId={activeServerId} />
        </OverlayPanel>
      )}

      {/* Voice settings */}
      {voiceSettingsOpen && (
        <OverlayPanel onClose={() => setVoiceSettingsOpen(false)} title="Voice Settings">
          <VoiceSettingsPanel onClose={() => setVoiceSettingsOpen(false)} />
        </OverlayPanel>
      )}

      {/* Onboarding wizard — auto-show on first launch */}
      {showOnboarding && (
        <OverlayPanel onClose={() => {}} title="Getting started" hideClose>
          <OnboardingWizard />
        </OverlayPanel>
      )}
    </div>
  );
}

// ── Shared overlay shell ────────────────────────────────────────────────────

function OverlayPanel({
  children,
  onClose,
  title,
  hideClose = false,
}: {
  children: React.ReactNode;
  onClose: () => void;
  title: string;
  hideClose?: boolean;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onClick={(e) => { if (e.target === e.currentTarget && !hideClose) onClose(); }}
    >
      <div className="relative bg-bg-800 border border-bg-600/50 rounded-xl shadow-2xl w-full max-w-3xl max-h-[85vh] flex flex-col overflow-hidden mx-4">
        {/* Panel header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-bg-600/40 shrink-0">
          <h2 className="text-sm font-semibold text-fg">{title}</h2>
          {!hideClose && (
            <button
              onClick={onClose}
              aria-label="Close panel"
              className="text-muted hover:text-fg transition-colors p-1 rounded"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
              </svg>
            </button>
          )}
        </div>
        {/* Panel content */}
        <div className="flex-1 overflow-y-auto">
          {children}
        </div>
      </div>
    </div>
  );
}

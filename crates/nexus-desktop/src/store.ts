import { create } from "zustand";
import { invoke } from "./invoke";
import type { PluginManifest } from "./plugins/types";
import { DEFAULT_THEME_ID } from "./themes/themes";

export interface Session {
  userId: string;
  username: string;
  displayName?: string;
  avatar?: string;
  serverUrl: string;
  accessToken: string;
}

export interface Server {
  id: string;
  name: string;
  icon?: string;
  ownerId: string;
  require2fa?: boolean;
  spamWindowSecs?: number;
  spamMaxMessages?: number;
  /** v0.15 — current supporter tier (0-3) */
  boostTier?: number;
  /** v0.15 — active booster count */
  boosterCount?: number;
  /** v0.15 — vanity invite code */
  vanityCode?: string;
  /** v0.16 — discovery tags */
  tags?: string[];
  /** v0.16 — discovery category */
  category?: string;
  /** v0.16 — popularity score */
  activityScore?: number;
  /** v0.16 — when featured (null = not featured) */
  featuredAt?: string;
  /** v0.16 — external donation/tip URL */
  tipJarUrl?: string;
  /** member count */
  memberCount?: number;
  description?: string;
}

export interface Channel {
  id: string;
  serverId: string;
  name: string;
  kind: "text" | "voice" | "announcement";
  isE2ee?: boolean;
  disappearAfterSeconds?: number;
  /** v0.14 — stream/topic-threaded channel */
  isStream?: boolean;
}

export interface Message {
  id: string;
  channelId: string;
  authorId: string;
  authorUsername: string;
  content: string;
  createdAt: string;
  editedAt?: string;
  attachments?: Attachment[];
  reactions?: Reaction[];
  embeds?: Embed[];
  replyTo?: string;
  threadId?: string;
  /** v0.14 */
  forwardedFromMessageId?: string;
  forwardedFromChannelId?: string;
  stickerIds?: string[];
  topic?: string;
}

export interface Attachment {
  id: string;
  filename: string;
  url: string;
  size: number;
  contentType?: string;
}

export interface Reaction {
  emoji: string;
  count: number;
  /** true if the current user has added this reaction */
  me: boolean;
}

export interface Embed {
  type?: string;
  title?: string;
  description?: string;
  url?: string;
  thumbnail?: { url: string };
  image?: { url: string };
  color?: number;
}

export interface VoiceParticipant {
  userId: string;
  username: string;
  speaking: boolean;
  muted: boolean;
  deafened: boolean;
  avatar?: string;
}

export interface Relationship {
  /** The other user's ID */
  id: string;
  direction: "incoming" | "outgoing";
  status: "pending" | "accepted" | "blocked" | "denied";
  user: {
    id: string;
    username: string;
    displayName?: string;
    avatar?: string;
  };
}

export interface DmChannel {
  id: string;
  channelType: "dm" | "group_dm";
  name?: string;
  lastMessageId?: string;
  recipients: {
    id: string;
    username: string;
    displayName?: string;
    avatar?: string;
  }[];
}

export interface InAppNotification {
  id: string;
  channelId: string;
  channelName?: string;
  authorId: string;
  authorUsername: string;
  content: string;
  createdAt: string;
  read: boolean;
}

export interface ServerMember {
  userId: string;
  username: string;
  displayName?: string;
  avatar?: string;
  nickname?: string;
  presence: "online" | "idle" | "do_not_disturb" | "invisible" | "offline";
  joinedAt: string;
  roles: string[];
  muted: boolean;
  deafened: boolean;
}

export interface UpdateInfo {
  version: string;
  body: string;
}

// ── Phase 13 engagement types ────────────────────────────────────────────────

export interface Poll {
  id: string;
  channelId: string;
  messageId?: string;
  authorId: string;
  question: string;
  options: string[];
  endsAt?: string;
  allowMultiselect: boolean;
  isAnonymous: boolean;
  status: "open" | "ended";
  createdAt: string;
  updatedAt: string;
}

export interface PollOptionResult {
  index: number;
  label: string;
  voteCount: number;
  voterIds: string[];
}

export interface PollResults {
  poll: Poll;
  options: PollOptionResult[];
  totalVoters: number;
}

export interface ScheduledMessage {
  id: string;
  channelId: string;
  authorId: string;
  content: string;
  scheduledAt: string;
  status: "pending" | "sent" | "failed" | "cancelled";
  createdAt: string;
}

export interface BookmarkedMessagePreview {
  id: string;
  content: string;
  authorId: string;
  authorUsername: string;
  channelId: string;
  createdAt: string;
}

export interface Bookmark {
  id: string;
  userId: string;
  messageId: string;
  channelId: string;
  note?: string;
  createdAt: string;
  message?: BookmarkedMessagePreview;
}

// ─── v0.14 Platform Differentiation types ────────────────────────────────────

export type ServerEventStatus = "scheduled" | "active" | "completed" | "cancelled";

export interface ServerEvent {
  id: string;
  serverId: string;
  creatorId: string;
  title: string;
  description?: string;
  startsAt: string;
  endsAt?: string;
  location?: string;
  channelId?: string;
  coverImage?: string;
  status: ServerEventStatus;
  interestedCount: number;
  isInterested: boolean;
  createdAt: string;
  updatedAt: string;
}

export type StickerType = "png" | "apng" | "lottie";

export interface Sticker {
  id: string;
  packId: string;
  serverId?: string;
  name: string;
  description?: string;
  assetUrl: string;
  type: StickerType;
  createdAt: string;
}

export interface StickerPack {
  id: string;
  name: string;
  description?: string;
  coverStickerId?: string;
  serverId?: string;
  isPremium: boolean;
  stickers?: Sticker[];
}

export interface InlineSuggestion {
  title: string;
  description?: string;
  content: string;
  previewUrl?: string;
}

// ── v0.15 Community Ecosystem ────────────────────────────────────────────────

export interface UserBadge {
  id: string;
  userId: string;
  badgeType: string;
  serverId?: string;
  awardedBy?: string;
  awardedAt: string;
  label?: string;
  iconUrl?: string;
}

export interface BoosterEntry {
  id: string;
  userId: string;
  serverId: string;
  slot: number;
  startedAt: string;
  expiresAt?: string;
}

export interface BoostTierInfo {
  tier: number;
  boosterCount: number;
  extraEmojiSlots: number;
  uploadLimitBytes: number;
  vanityUrlAvailable: boolean;
  currentVanityCode?: string;
}

export type CanvasBlockType = "heading" | "paragraph" | "image" | "code" | "divider" | "table" | "callout";

export interface CanvasBlock {
  id: string;
  channelId: string;
  blockType: CanvasBlockType;
  content: Record<string, unknown>;
  position: number;
  updatedBy?: string;
  updatedAt: string;
}

// ── Federation types (v0.8.5) ─────────────────────────────────────────────────

export type FederationPolicy = "open" | "closed" | "invite_only";

export interface FederationStatus {
  serverName: string;
  softwareVersion: string;
  federationEnabled: boolean;
  peerCount: number;
  healthyPeerCount: number;
  pendingInboundRequests: number;
  pendingOutboundRequests: number;
  uptimeSeconds: number;
}

export interface FederationIdentity {
  displayName?: string;
  description?: string;
  adminContact?: string;
  federationPolicy: FederationPolicy;
}

export interface FederatedPeer {
  domain: string;
  displayName?: string;
  trustScore: number;
  isHealthy: boolean;
  isBlocked: boolean;
  latencyMs?: number;
  lastSeenAt?: string;
  notes?: string;
  addedAt: string;
}

export interface PeeringRequest {
  id: string;
  direction: "inbound" | "outbound";
  remoteDomain: string;
  remoteDisplayName?: string;
  remoteDescription?: string;
  status: "pending" | "accepted" | "rejected" | "cancelled";
  message?: string;
  createdAt: string;
}

export interface FederationAuditEntry {
  id: string;
  adminId: string;
  action: string;
  targetDomain: string;
  details: Record<string, unknown>;
  createdAt: string;
}

// ─── v1.5 Collaboration & Productivity ────────────────────────────────────────

export type TaskStatus = "open" | "in_progress" | "done" | "cancelled";
export type TaskPriority = "low" | "medium" | "high" | "urgent";

export interface Task {
  id: string;
  serverId: string;
  channelId: string;
  creatorId: string;
  assigneeId?: string;
  title: string;
  description?: string;
  status: TaskStatus;
  priority: TaskPriority;
  dueAt?: string;
  completedAt?: string;
  position: number;
  createdAt: string;
  updatedAt: string;
}

export interface ChecklistItem {
  id: string;
  taskId: string;
  content: string;
  checked: boolean;
  position: number;
  createdAt: string;
}

export interface CalendarEvent {
  id: string;
  serverId: string;
  channelId?: string;
  creatorId: string;
  title: string;
  description?: string;
  location?: string;
  startsAt: string;
  endsAt: string;
  allDay: boolean;
  rrule?: string;
  color?: string;
  createdAt: string;
  updatedAt: string;
}

export type RsvpStatus = "going" | "interested" | "declined";

export interface CalendarRsvp {
  eventId: string;
  userId: string;
  status: RsvpStatus;
  respondedAt: string;
}

export interface FileVersion {
  id: string;
  attachmentId: string;
  uploaderId: string;
  versionNumber: number;
  filename: string;
  contentType?: string;
  size: number;
  storageKey: string;
  sha256?: string;
  comment?: string;
  createdAt: string;
}

export interface ServerStorageQuota {
  serverId: string;
  maxBytes: number;
  usedBytes: number;
  updatedAt: string;
}

export type DigestInterval = "daily" | "weekly";

export interface AiPreferences {
  userId: string;
  summariesEnabled: boolean;
  smartReplies: boolean;
  autoModSuggest: boolean;
  digestEnabled: boolean;
  digestInterval: DigestInterval;
  updatedAt: string;
}

export interface ChannelDigest {
  id: string;
  channelId: string;
  userId: string;
  periodStart: string;
  periodEnd: string;
  summary: string;
  messageCount: number;
  createdAt: string;
}

// ─── v1.6 Multimedia & Expression ─────────────────────────────────────────

export interface VoiceNote {
  id: string;
  channelId: string;
  authorId: string;
  storageKey: string;
  filename: string;
  contentType: string;
  size: number;
  durationMs: number;
  waveform: string | null;
  transcript: string | null;
  messageId: string | null;
  createdAt: string;
}

export interface VideoNote {
  id: string;
  channelId: string;
  authorId: string;
  storageKey: string;
  filename: string;
  contentType: string;
  size: number;
  durationMs: number;
  width: number | null;
  height: number | null;
  thumbnailKey: string | null;
  transcript: string | null;
  messageId: string | null;
  createdAt: string;
}

export interface Story {
  id: string;
  authorId: string;
  mediaType: string;
  storageKey: string | null;
  textContent: string | null;
  textStyle: Record<string, unknown> | null;
  expiresAt: string;
  visibility: string;
  createdAt: string;
}

export interface StoryView {
  storyId: string;
  viewerId: string;
  viewedAt: string;
}

export interface Drawing {
  id: string;
  channelId: string;
  authorId: string;
  drawingData: Record<string, unknown>;
  width: number;
  height: number;
  previewKey: string | null;
  messageId: string | null;
  isWhiteboard: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface VoiceSettings {
  userId: string;
  spatialAudio: boolean;
  noiseGateEnabled: boolean;
  noiseGateThreshold: number;
  echoCancelLevel: string;
  autoGain: boolean;
  updatedAt: string;
}

export interface VoiceMusicQueueItem {
  id: string;
  channelId: string;
  addedBy: string;
  title: string;
  sourceUrl: string;
  durationMs: number;
  position: number;
  status: string;
  createdAt: string;
}

export interface MediaGalleryFilter {
  id: string;
  userId: string;
  serverId: string | null;
  channelId: string | null;
  name: string;
  mediaTypes: string[];
  dateFrom: string | null;
  dateTo: string | null;
  createdAt: string;
}

// ── v1.7 Accessibility & Inclusivity ──────────────────────────────────────────

export interface UserAccessibilitySettings {
  userId: string;
  screenReaderMode: boolean;
  announceMessages: boolean;
  announceReactions: boolean;
  announceTyping: boolean;
  keyboardShortcuts: boolean;
  highContrastMode: boolean;
  reducedMotion: boolean;
  fontFamily: string;
  customFontName: string | null;
  colorBlindMode: string;
  preferredLanguage: string;
  autoTranslate: boolean;
  rtlOverride: boolean;
  captionsEnabled: boolean;
  captionFontSize: string;
  captionPosition: string;
  ttsEnabled: boolean;
  ttsRate: number;
  ttsVoice: string;
  createdAt: string;
  updatedAt: string;
}

export interface VoiceCaption {
  id: string;
  channelId: string;
  speakerId: string;
  text: string;
  language: string;
  isFinal: boolean;
  startedAt: string;
  endedAt: string | null;
  createdAt: string;
}

export interface MessageTranslation {
  id: string;
  messageId: string;
  sourceLanguage: string;
  targetLanguage: string;
  translatedText: string;
  createdAt: string;
}

export interface MessageTtsRequest {
  id: string;
  userId: string;
  messageId: string;
  channelId: string;
  status: string;
  createdAt: string;
}

// ─── v1.8 Ecosystem & Onboarding ──────────────────────────────────────────────

export interface ImportJob {
  id: string;
  serverId: string;
  userId: string;
  sourcePlatform: string;
  status: string;
  totalItems: number;
  importedItems: number;
  errorLog: string | null;
  metadata: unknown;
  createdAt: string;
  updatedAt: string;
}

export interface BulkInvitation {
  id: string;
  serverId: string;
  inviterId: string;
  emails: unknown;
  status: string;
  sentCount: number;
  totalCount: number;
  inviteCode: string;
  createdAt: string;
}

export interface ServerTemplate {
  id: string;
  name: string;
  description: string | null;
  category: string;
  iconUrl: string | null;
  channels: unknown;
  roles: unknown;
  settings: unknown;
  isBuiltin: boolean;
  creatorId: string | null;
  usageCount: number;
  createdAt: string;
}

export interface OnboardingProgress {
  userId: string;
  completedSteps: unknown;
  dismissed: boolean;
}

export interface AnalyticsSnapshot {
  id: string;
  serverId: string;
  periodDate: string;
  messagesCount: number;
  activeMembers: number;
  newMembers: number;
  leftMembers: number;
  voiceMinutes: number;
  reportsResolved: number;
  bansIssued: number;
  filtersTriggered: number;
  createdAt: string;
}

export interface MarketplacePlugin {
  id: string;
  name: string;
  slug: string;
  description: string | null;
  authorId: string | null;
  version: string;
  manifestUrl: string;
  iconUrl: string | null;
  sourceUrl: string | null;
  signature: string | null;
  signingKeyId: string | null;
  category: string;
  tags: string[];
  downloads: number;
  avgRating: number;
  ratingCount: number;
  isVerified: boolean;
  isPublished: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface PluginReview {
  id: string;
  pluginId: string;
  userId: string;
  rating: number;
  title: string | null;
  body: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface PluginInstall {
  id: string;
  pluginId: string;
  serverId: string;
  installedBy: string;
  version: string;
  isEnabled: boolean;
  createdAt: string;
}

interface StoreState {
  // Auth
  session: Session | null;
  setSession: (session: Session | null) => void;

  // Servers
  servers: Server[];
  activeServerId: string | null;
  setServers: (servers: Server[]) => void;
  setActiveServer: (id: string | null) => void;

  // Channels
  channels: Channel[];
  activeChannelId: string | null;
  setChannels: (channels: Channel[]) => void;
  setActiveChannel: (id: string | null) => void;

  // Messages — keyed by channelId
  messages: Record<string, Message[]>;
  appendMessage: (channelId: string, msg: Message) => void;
  prependMessages: (channelId: string, msgs: Message[]) => void;
  setMessages: (channelId: string, msgs: Message[]) => void;
  updateMessageReaction: (channelId: string, messageId: string, emoji: string, delta: number, mine: boolean) => void;

  // Typing — keyed by channelId → list of usernames currently typing
  typingUsers: Record<string, string[]>;
  setTyping: (channelId: string, username: string, active: boolean) => void;

  // Unread — channelIds that have received messages since last viewed
  unreadChannels: Record<string, boolean>;

  // Voice
  voiceParticipants: VoiceParticipant[];
  joinedVoiceChannelId: string | null;
  pttActive: boolean;
  setVoiceParticipants: (participants: VoiceParticipant[]) => void;
  setJoinedVoiceChannel: (id: string | null) => void;
  setPttActive: (active: boolean) => void;

  // UI
  updateAvailable: UpdateInfo | null;
  setUpdateAvailable: (info: UpdateInfo | null) => void;
  sidebarCollapsed: boolean;
  setSidebarCollapsed: (v: boolean) => void;

  // Theme
  activeThemeId: string;
  setActiveTheme: (id: string) => void;

  // Plugins
  plugins: PluginManifest[];
  enabledPlugins: string[];
  installPlugin: (manifest: PluginManifest) => void;
  uninstallPlugin: (id: string) => void;
  togglePlugin: (id: string) => void;

  // Actions
  logout: () => Promise<void>;
  loadServers: () => Promise<void>;
  loadChannels: (serverId: string) => Promise<void>;
  loadMessages: (channelId: string, before?: string) => Promise<void>;
  createChannel: (serverId: string, name: string, channelType: string) => Promise<Channel>;

  // Home / Friends / DMs
  isHomeMode: boolean;
  setHomeMode: (v: boolean) => void;
  relationships: Relationship[];
  setRelationships: (rels: Relationship[]) => void;
  dmChannels: DmChannel[];
  setDmChannels: (dms: DmChannel[]) => void;
  appendDmChannel: (dm: DmChannel) => void;
  loadRelationships: () => Promise<void>;
  loadDmChannels: () => Promise<void>;

  // In-app notifications (@mentions)
  inAppNotifications: InAppNotification[];
  unreadNotificationCount: number;
  addInAppNotification: (notif: Omit<InAppNotification, 'read'>) => void;
  markNotificationsRead: () => void;
  clearNotifications: () => void;

  // Server members — keyed by serverId
  members: Record<string, ServerMember[]>;
  loadMembers: (serverId: string) => Promise<void>;

  // Polls — keyed by channelId
  channelPolls: Record<string, Poll[]>;
  setChannelPolls: (channelId: string, polls: Poll[]) => void;
  updatePoll: (channelId: string, pollId: string, updater: (p: Poll) => Poll) => void;

  // Poll results — keyed by pollId
  pollResults: Record<string, PollResults>;
  setPollResults: (pollId: string, results: PollResults) => void;

  // Drafts (mirrors localStorage for UI reactivity)
  drafts: Record<string, string>;
  setDraft: (channelId: string, text: string) => void;

  // Bookmarks
  bookmarks: Bookmark[];
  setBookmarks: (bookmarks: Bookmark[]) => void;
  addBookmark: (bookmark: Bookmark) => void;
  removeBookmark: (messageId: string) => void;
  loadBookmarks: () => Promise<void>;

  // Note-to-self / Saved Notes
  noteToSelfChannelId: string | null;
  setNoteToSelfChannelId: (id: string | null) => void;
  loadNoteToSelfChannel: () => Promise<void>;

  // Saved Messages panel visibility
  savedMessagesPanelOpen: boolean;
  setSavedMessagesPanelOpen: (v: boolean) => void;

  // Scheduled messages — keyed by channelId
  channelScheduled: Record<string, ScheduledMessage[]>;
  setChannelScheduled: (channelId: string, msgs: ScheduledMessage[]) => void;

  // ─── v0.14 Platform Differentiation ───────────────────────────────────

  // Message forwarding — the message being forwarded (drives ForwardModal)
  forwardModalMessage: Message | null;
  setForwardModalMessage: (msg: Message | null) => void;

  // Server events — keyed by serverId
  serverEvents: Record<string, ServerEvent[]>;
  setServerEvents: (serverId: string, events: ServerEvent[]) => void;
  upsertServerEvent: (serverId: string, event: ServerEvent) => void;
  removeServerEvent: (serverId: string, eventId: string) => void;
  eventsOpen: boolean;
  setEventsOpen: (open: boolean) => void;
  loadServerEvents: (serverId: string, status?: string) => Promise<void>;

  // Sticker packs (global)
  stickerPacks: StickerPack[];
  setStickerPacks: (packs: StickerPack[]) => void;
  loadStickerPacks: () => Promise<void>;

  // Server stickers — keyed by serverId
  serverStickers: Record<string, Sticker[]>;
  setServerStickers: (serverId: string, stickers: Sticker[]) => void;
  loadServerStickers: (serverId: string) => Promise<void>;

  // ─── v0.15 Community Ecosystem ────────────────────────────────────────

  // User badges — keyed by userId
  userBadges: Record<string, UserBadge[]>;
  setUserBadges: (userId: string, badges: UserBadge[]) => void;
  loadUserBadges: (userId: string) => Promise<void>;

  // Server boost tier — keyed by serverId
  boostTierInfo: Record<string, BoostTierInfo>;
  setBoostTierInfo: (serverId: string, info: BoostTierInfo) => void;
  loadBoostTierInfo: (serverId: string) => Promise<void>;

  // Canvas blocks — keyed by channelId
  canvasBlocks: Record<string, CanvasBlock[]>;
  setCanvasBlocks: (channelId: string, blocks: CanvasBlock[]) => void;
  upsertCanvasBlock: (channelId: string, block: CanvasBlock) => void;
  removeCanvasBlock: (channelId: string, blockId: string) => void;
  loadCanvasBlocks: (channelId: string) => Promise<void>;

  // Federation (v0.8.5) — instance-admin management
  federationStatus: FederationStatus | null;
  loadFederationStatus: () => Promise<void>;
  federationIdentity: FederationIdentity | null;
  loadFederationIdentity: () => Promise<void>;
  federatedPeers: FederatedPeer[];
  loadFederatedPeers: () => Promise<void>;
  peeringRequests: PeeringRequest[];
  loadPeeringRequests: (status?: string) => Promise<void>;

  // ─── v1.5 Collaboration & Productivity ────────────────────────────────

  // Tasks — keyed by channelId
  channelTasks: Record<string, Task[]>;
  setChannelTasks: (channelId: string, tasks: Task[]) => void;
  upsertTask: (channelId: string, task: Task) => void;
  removeTask: (channelId: string, taskId: string) => void;
  loadChannelTasks: (channelId: string, serverId: string) => Promise<void>;

  // Calendar events — keyed by serverId
  calendarEvents: Record<string, CalendarEvent[]>;
  setCalendarEvents: (serverId: string, events: CalendarEvent[]) => void;
  upsertCalendarEvent: (serverId: string, event: CalendarEvent) => void;
  removeCalendarEvent: (serverId: string, eventId: string) => void;
  loadCalendarEvents: (serverId: string, from?: string, to?: string) => Promise<void>;

  // File versions — keyed by attachmentId
  fileVersions: Record<string, FileVersion[]>;
  setFileVersions: (attachmentId: string, versions: FileVersion[]) => void;
  loadFileVersions: (attachmentId: string) => Promise<void>;

  // AI preferences
  aiPreferences: AiPreferences | null;
  setAiPreferences: (prefs: AiPreferences) => void;
  loadAiPreferences: () => Promise<void>;

  // ─── v1.6 Multimedia & Expression ─────────────────────────────────────

  // Voice notes — keyed by channelId
  channelVoiceNotes: Record<string, VoiceNote[]>;
  loadVoiceNotes: (channelId: string) => Promise<void>;

  // Video notes — keyed by channelId
  channelVideoNotes: Record<string, VideoNote[]>;
  loadVideoNotes: (channelId: string) => Promise<void>;

  // Stories — feed of active stories
  storyFeed: Story[];
  loadStoryFeed: () => Promise<void>;

  // Drawings — keyed by channelId
  channelDrawings: Record<string, Drawing[]>;
  loadDrawings: (channelId: string) => Promise<void>;

  // Voice settings (per-user)
  voiceSettings: VoiceSettings | null;
  setVoiceSettings: (settings: VoiceSettings) => void;
  loadVoiceSettings: () => Promise<void>;

  // Music queue — keyed by channelId
  musicQueue: Record<string, VoiceMusicQueueItem[]>;
  loadMusicQueue: (channelId: string) => Promise<void>;

  // Media gallery filters
  mediaGalleryFilters: MediaGalleryFilter[];
  loadMediaGalleryFilters: () => Promise<void>;

  // Accessibility settings
  accessibilitySettings: UserAccessibilitySettings | null;
  loadAccessibilitySettings: () => Promise<void>;
  setAccessibilitySettings: (settings: UserAccessibilitySettings) => void;

  // Voice captions — keyed by channelId
  voiceCaptions: Record<string, VoiceCaption[]>;
  loadVoiceCaptions: (channelId: string) => Promise<void>;
  addVoiceCaption: (caption: VoiceCaption) => void;

  // ─── v1.8 Ecosystem & Onboarding ───────────────────────────────────────

  // Import jobs — keyed by serverId
  importJobs: Record<string, ImportJob[]>;
  loadImportJobs: (serverId: string) => Promise<void>;

  // Onboarding progress (current user)
  onboardingProgress: OnboardingProgress | null;
  loadOnboardingProgress: () => Promise<void>;
  dismissOnboarding: () => Promise<void>;

  // Server templates
  serverTemplates: ServerTemplate[];
  loadServerTemplates: (category?: string) => Promise<void>;

  // Analytics snapshots — keyed by serverId
  analyticsSnapshots: Record<string, AnalyticsSnapshot[]>;
  loadAnalyticsSnapshots: (serverId: string, days?: number) => Promise<void>;

  // Marketplace plugins (browse results)
  marketplacePlugins: MarketplacePlugin[];
  loadMarketplacePlugins: (query?: string, category?: string) => Promise<void>;

  // Plugin reviews — keyed by pluginId
  pluginReviews: Record<string, PluginReview[]>;
  loadPluginReviews: (pluginId: string) => Promise<void>;

  // Server plugin installs — keyed by serverId
  serverPluginInstalls: Record<string, PluginInstall[]>;
  loadServerPluginInstalls: (serverId: string) => Promise<void>;
}

// Module-level map so typing-clear timeouts survive re-renders
const _typingTimers = new Map<string, ReturnType<typeof setTimeout>>();

// ── Session persistence ───────────────────────────────────────────────────────
const SESSION_KEY = "nexus:session";

function loadPersistedSession(): Session | null {
  try {
    const raw = localStorage.getItem(SESSION_KEY);
    if (raw) return JSON.parse(raw) as Session;
  } catch { /* corrupt data — ignore */ }
  return null;
}

function persistSession(session: Session | null) {
  if (session) {
    localStorage.setItem(SESSION_KEY, JSON.stringify(session));
  } else {
    localStorage.removeItem(SESSION_KEY);
  }
}

export const useStore = create<StoreState>((set, get) => ({
  // ─── Auth ─────────────────────────────────────────────────────────────
  session: loadPersistedSession(),
  setSession: (session) => {
    persistSession(session);
    set({ session });
  },

  // ─── Servers ──────────────────────────────────────────────────────────
  servers: [],
  activeServerId: null,
  setServers: (servers) => set({ servers }),
  setActiveServer: (id) => set({ activeServerId: id, channels: [], activeChannelId: null }),

  // ─── Channels ─────────────────────────────────────────────────────────
  channels: [],
  activeChannelId: null,
  setChannels: (channels) => set({ channels }),
  setActiveChannel: (id) => set((s) => {
    const unreadChannels = { ...s.unreadChannels };
    if (id) delete unreadChannels[id];
    return { activeChannelId: id, unreadChannels };
  }),

  // ─── Messages ─────────────────────────────────────────────────────────
  messages: {},
  appendMessage: (channelId, msg) =>
    set((s) => {
      const existing = s.messages[channelId] ?? [];
      // Dedup: if message already present (optimistic add + WS event), skip
      if (existing.some((m) => m.id === msg.id)) return s;
      return {
        messages: {
          ...s.messages,
          [channelId]: [...existing, msg],
        },
        // Mark channel unread if the user isn't currently looking at it
        unreadChannels: s.activeChannelId === channelId
          ? s.unreadChannels
          : { ...s.unreadChannels, [channelId]: true },
      };
    }),
  prependMessages: (channelId, msgs) =>
    set((s) => ({
      messages: {
        ...s.messages,
        [channelId]: [...msgs, ...(s.messages[channelId] ?? [])],
      },
    })),
  setMessages: (channelId, msgs) =>
    set((s) => ({
      messages: { ...s.messages, [channelId]: msgs },
    })),

  updateMessageReaction: (channelId, messageId, emoji, delta, mine) =>
    set((s) => {
      const msgs = s.messages[channelId];
      if (!msgs) return s;
      return {
        messages: {
          ...s.messages,
          [channelId]: msgs.map((m) => {
            if (m.id !== messageId) return m;
            const existing = m.reactions ?? [];
            const idx = existing.findIndex((r) => r.emoji === emoji);
            if (idx >= 0) {
              const updated = [...existing];
              const newCount = updated[idx].count + delta;
              if (newCount <= 0) {
                updated.splice(idx, 1);
              } else {
                updated[idx] = {
                  ...updated[idx],
                  count: newCount,
                  me: delta > 0 ? mine || updated[idx].me : (mine ? false : updated[idx].me),
                };
              }
              return { ...m, reactions: updated };
            } else if (delta > 0) {
              return { ...m, reactions: [...existing, { emoji, count: 1, me: mine }] };
            }
            return m;
          }),
        },
      };
    }),

  // ─── Typing ───────────────────────────────────────────────────────────────
  typingUsers: {},
  setTyping: (channelId, username, active) => {
    const key = `${channelId}:${username}`;
    if (active) {
      set((s) => ({
        typingUsers: {
          ...s.typingUsers,
          [channelId]: [...new Set([...(s.typingUsers[channelId] ?? []), username])],
        },
      }));
      if (_typingTimers.has(key)) clearTimeout(_typingTimers.get(key)!);
      _typingTimers.set(key, setTimeout(() => {
        set((s) => ({
          typingUsers: {
            ...s.typingUsers,
            [channelId]: (s.typingUsers[channelId] ?? []).filter((u) => u !== username),
          },
        }));
        _typingTimers.delete(key);
      }, 6000));
    } else {
      if (_typingTimers.has(key)) { clearTimeout(_typingTimers.get(key)!); _typingTimers.delete(key); }
      set((s) => ({
        typingUsers: {
          ...s.typingUsers,
          [channelId]: (s.typingUsers[channelId] ?? []).filter((u) => u !== username),
        },
      }));
    }
  },

  // ─── Unread ───────────────────────────────────────────────────────────────
  unreadChannels: {},

  // ─── Voice ────────────────────────────────────────────────────────────
  voiceParticipants: [],
  joinedVoiceChannelId: null,
  pttActive: false,
  setVoiceParticipants: (participants) => set({ voiceParticipants: participants }),
  setJoinedVoiceChannel: (id) => set({ joinedVoiceChannelId: id }),
  setPttActive: (active) => set({ pttActive: active }),

  // ─── UI ───────────────────────────────────────────────────────────────
  updateAvailable: null,
  setUpdateAvailable: (info) => set({ updateAvailable: info }),
  sidebarCollapsed: false,
  setSidebarCollapsed: (v) => set({ sidebarCollapsed: v }),

  // ─── Theme ────────────────────────────────────────────────────────────
  activeThemeId: localStorage.getItem("nexus:theme") ?? DEFAULT_THEME_ID,
  setActiveTheme: (id) => {
    localStorage.setItem("nexus:theme", id);
    set({ activeThemeId: id });
  },

  // ─── Plugins ──────────────────────────────────────────────────────────
  plugins: (() => {
    try {
      return JSON.parse(localStorage.getItem("nexus:plugins") ?? "[]") as PluginManifest[];
    } catch { return []; }
  })(),
  enabledPlugins: (() => {
    try {
      return JSON.parse(localStorage.getItem("nexus:enabled-plugins") ?? "[]") as string[];
    } catch { return []; }
  })(),
  installPlugin: (manifest) =>
    set((s) => {
      const plugins = [...s.plugins.filter((p) => p.id !== manifest.id), manifest];
      localStorage.setItem("nexus:plugins", JSON.stringify(plugins));
      return { plugins };
    }),
  uninstallPlugin: (id) =>
    set((s) => {
      const plugins = s.plugins.filter((p) => p.id !== id);
      const enabledPlugins = s.enabledPlugins.filter((i) => i !== id);
      localStorage.setItem("nexus:plugins", JSON.stringify(plugins));
      localStorage.setItem("nexus:enabled-plugins", JSON.stringify(enabledPlugins));
      return { plugins, enabledPlugins };
    }),
  togglePlugin: (id) =>
    set((s) => {
      const enabledPlugins = s.enabledPlugins.includes(id)
        ? s.enabledPlugins.filter((i) => i !== id)
        : [...s.enabledPlugins, id];
      localStorage.setItem("nexus:enabled-plugins", JSON.stringify(enabledPlugins));
      return { enabledPlugins };
    }),

  // ─── Actions ──────────────────────────────────────────────────────────
  logout: async () => {
    try {
      await invoke("logout");
    } catch {
      // ignore
    }
    _typingTimers.forEach(clearTimeout);
    _typingTimers.clear();
    persistSession(null);
    set({
      session: null,
      servers: [],
      channels: [],
      messages: {},
      typingUsers: {},
      unreadChannels: {},
      activeServerId: null,
      activeChannelId: null,
      isHomeMode: false,
      relationships: [],
      dmChannels: [],
      members: {},
      inAppNotifications: [],
      unreadNotificationCount: 0,
      channelPolls: {},
      pollResults: {},
      drafts: {},
      bookmarks: [],
      noteToSelfChannelId: null,
      savedMessagesPanelOpen: false,
      channelScheduled: {},
    });
  },

  loadServers: async () => {
    try {
      const servers = await invoke<Server[]>("list_servers");
      set({ servers });
      // Auto-select the first server if none is currently active
      if (servers.length > 0 && !get().activeServerId) {
        get().setActiveServer(servers[0].id);
      }
    } catch (e) {
      // If the token expired and couldn't be refreshed, force logout
      if (e instanceof Error && e.message.startsWith("401:")) {
        console.warn("Session expired — logging out");
        get().logout();
        return;
      }
      console.error("loadServers error", e);
    }
  },

  loadChannels: async (serverId: string) => {
    try {
      const channels = await invoke<Channel[]>("list_channels", { serverId });
      set({ channels });
    } catch (e) {
      console.error("loadChannels error", e);
    }
  },

  createChannel: async (serverId: string, name: string, channelType: string): Promise<Channel> => {
    const ch = await invoke<Channel>("create_channel", { serverId, name, channelType });
    set((s) => ({ channels: [...s.channels, ch] }));
    return ch;
  },

  loadMessages: async (channelId: string, before?: string) => {
    try {
      const msgs = await invoke<Message[]>("fetch_history", {
        channelId,
        before: before ?? null,
        limit: 50,
      });
      if (before) {
        get().prependMessages(channelId, msgs);
      } else {
        get().setMessages(channelId, msgs);
      }
    } catch (e) {
      console.error("loadMessages error", e);
    }
  },

  // ─── Home / Friends / DMs ────────────────────────────────────────────
  isHomeMode: false,
  setHomeMode: (v) => set({ isHomeMode: v }),
  relationships: [],
  setRelationships: (rels) => set({ relationships: rels }),
  dmChannels: [],
  setDmChannels: (dms) => set({ dmChannels: dms }),
  appendDmChannel: (dm) =>
    set((s) => {
      if (s.dmChannels.some((d) => d.id === dm.id)) return s;
      return { dmChannels: [dm, ...s.dmChannels] };
    }),

  loadRelationships: async () => {
    try {
      const rels = await invoke<Relationship[]>("list_relationships");
      set({ relationships: rels });
    } catch (e) {
      console.error("loadRelationships error", e);
    }
  },

  loadDmChannels: async () => {
    try {
      const dms = await invoke<DmChannel[]>("list_dm_channels");
      set({ dmChannels: dms });
    } catch (e) {
      console.error("loadDmChannels error", e);
    }
  },

  // ─── In-app notifications ─────────────────────────────────────────────
  inAppNotifications: [],
  unreadNotificationCount: 0,
  addInAppNotification: (notif) =>
    set((s) => {
      const full: InAppNotification = { ...notif, read: false };
      // Cap at 100 most recent notifications
      const updated = [full, ...s.inAppNotifications].slice(0, 100);
      return {
        inAppNotifications: updated,
        unreadNotificationCount: s.unreadNotificationCount + 1,
      };
    }),
  markNotificationsRead: () =>
    set((s) => ({
      inAppNotifications: s.inAppNotifications.map((n) => ({ ...n, read: true })),
      unreadNotificationCount: 0,
    })),
  clearNotifications: () =>
    set({ inAppNotifications: [], unreadNotificationCount: 0 }),

  // ─── Server members ───────────────────────────────────────────────────
  members: {},
  loadMembers: async (serverId: string) => {
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const raw = await invoke<any[]>("list_members", { serverId });
      const mapped: ServerMember[] = raw.map((m) => ({
        userId: m.user_id,
        username: m.username,
        displayName: m.display_name ?? undefined,
        avatar: m.avatar ?? undefined,
        nickname: m.nickname ?? undefined,
        presence: m.presence ?? "offline",
        joinedAt: m.joined_at,
        roles: [],
        muted: m.muted ?? false,
        deafened: m.deafened ?? false,
      }));
      set((s) => ({ members: { ...s.members, [serverId]: mapped } }));
    } catch (e) {
      console.error("loadMembers error", e);
    }
  },

  // ─── Polls ────────────────────────────────────────────────────────────
  channelPolls: {},
  setChannelPolls: (channelId, polls) =>
    set((s) => ({ channelPolls: { ...s.channelPolls, [channelId]: polls } })),
  updatePoll: (channelId, pollId, updater) =>
    set((s) => {
      const existing = s.channelPolls[channelId];
      if (!existing) return s;
      return {
        channelPolls: {
          ...s.channelPolls,
          [channelId]: existing.map((p) => (p.id === pollId ? updater(p) : p)),
        },
      };
    }),
  pollResults: {},
  setPollResults: (pollId, results) =>
    set((s) => ({ pollResults: { ...s.pollResults, [pollId]: results } })),

  // ─── Drafts ───────────────────────────────────────────────────────────
  drafts: (() => {
    // Re-hydrate from localStorage on first load
    const map: Record<string, string> = {};
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith("nexus:draft:")) {
        const channelId = key.slice("nexus:draft:".length);
        map[channelId] = localStorage.getItem(key) ?? "";
      }
    }
    return map;
  })(),
  setDraft: (channelId, text) => {
    if (text) {
      localStorage.setItem(`nexus:draft:${channelId}`, text);
    } else {
      localStorage.removeItem(`nexus:draft:${channelId}`);
    }
    set((s) => ({ drafts: { ...s.drafts, [channelId]: text } }));
  },

  // ─── Bookmarks ────────────────────────────────────────────────────────
  bookmarks: [],
  setBookmarks: (bookmarks) => set({ bookmarks }),
  addBookmark: (bookmark) =>
    set((s) => {
      if (s.bookmarks.some((b) => b.messageId === bookmark.messageId)) return s;
      return { bookmarks: [bookmark, ...s.bookmarks] };
    }),
  removeBookmark: (messageId) =>
    set((s) => ({ bookmarks: s.bookmarks.filter((b) => b.messageId !== messageId) })),
  loadBookmarks: async () => {
    try {
      const raw = await invoke<Bookmark[]>("list_bookmarks");
      set({ bookmarks: raw });
    } catch (e) {
      console.error("loadBookmarks error", e);
    }
  },

  // ─── Note-to-self ─────────────────────────────────────────────────────
  noteToSelfChannelId: null,
  setNoteToSelfChannelId: (id) => set({ noteToSelfChannelId: id }),
  loadNoteToSelfChannel: async () => {
    try {
      const res = await invoke<{ channelId: string }>("get_note_to_self_channel");
      set({ noteToSelfChannelId: res.channelId });
    } catch (e) {
      console.error("loadNoteToSelfChannel error", e);
    }
  },

  // ─── Saved Messages panel ─────────────────────────────────────────────
  savedMessagesPanelOpen: false,
  setSavedMessagesPanelOpen: (v) => set({ savedMessagesPanelOpen: v }),

  // ─── Scheduled messages ───────────────────────────────────────────────
  channelScheduled: {},
  setChannelScheduled: (channelId, msgs) =>
    set((s) => ({ channelScheduled: { ...s.channelScheduled, [channelId]: msgs } })),

  // ─── v0.14 Platform Differentiation ──────────────────────────────────
  forwardModalMessage: null,
  setForwardModalMessage: (msg) => set({ forwardModalMessage: msg }),

  serverEvents: {},
  setServerEvents: (serverId, events) =>
    set((s) => ({ serverEvents: { ...s.serverEvents, [serverId]: events } })),
  upsertServerEvent: (serverId, event) =>
    set((s) => {
      const existing = s.serverEvents[serverId] ?? [];
      const without = existing.filter((e) => e.id !== event.id);
      return { serverEvents: { ...s.serverEvents, [serverId]: [event, ...without] } };
    }),
  removeServerEvent: (serverId, eventId) =>
    set((s) => ({
      serverEvents: {
        ...s.serverEvents,
        [serverId]: (s.serverEvents[serverId] ?? []).filter((e) => e.id !== eventId),
      },
    })),
  eventsOpen: false,
  setEventsOpen: (open) => set({ eventsOpen: open }),
  loadServerEvents: async (serverId, status) => {
    try {
      const events = await invoke<ServerEvent[]>("list_server_events", { serverId, status });
      set((s) => ({ serverEvents: { ...s.serverEvents, [serverId]: events } }));
    } catch (e) {
      console.error("loadServerEvents error", e);
    }
  },

  stickerPacks: [],
  setStickerPacks: (packs) => set({ stickerPacks: packs }),
  loadStickerPacks: async () => {
    try {
      const packs = await invoke<StickerPack[]>("list_sticker_packs");
      set({ stickerPacks: packs });
    } catch (e) {
      console.error("loadStickerPacks error", e);
    }
  },

  serverStickers: {},
  setServerStickers: (serverId, stickers) =>
    set((s) => ({ serverStickers: { ...s.serverStickers, [serverId]: stickers } })),
  loadServerStickers: async (serverId) => {
    try {
      const stickers = await invoke<Sticker[]>("list_server_stickers", { serverId });
      set((s) => ({ serverStickers: { ...s.serverStickers, [serverId]: stickers } }));
    } catch (e) {
      console.error("loadServerStickers error", e);
    }
  },

  // ─── v0.15 Community Ecosystem ────────────────────────────────────────

  userBadges: {},
  setUserBadges: (userId, badges) =>
    set((s) => ({ userBadges: { ...s.userBadges, [userId]: badges } })),
  loadUserBadges: async (userId) => {
    try {
      const badges = await invoke<UserBadge[]>("list_user_badges", { userId });
      set((s) => ({ userBadges: { ...s.userBadges, [userId]: badges } }));
    } catch (e) {
      console.error("loadUserBadges error", e);
    }
  },

  boostTierInfo: {},
  setBoostTierInfo: (serverId, info) =>
    set((s) => ({ boostTierInfo: { ...s.boostTierInfo, [serverId]: info } })),
  loadBoostTierInfo: async (serverId) => {
    try {
      const info = await invoke<BoostTierInfo>("get_server_boost_tier", { serverId });
      set((s) => ({ boostTierInfo: { ...s.boostTierInfo, [serverId]: info } }));
    } catch (e) {
      console.error("loadBoostTierInfo error", e);
    }
  },

  canvasBlocks: {},
  setCanvasBlocks: (channelId, blocks) =>
    set((s) => ({ canvasBlocks: { ...s.canvasBlocks, [channelId]: blocks } })),
  upsertCanvasBlock: (channelId, block) =>
    set((s) => {
      const existing = s.canvasBlocks[channelId] ?? [];
      const without = existing.filter((b) => b.id !== block.id);
      const updated = [...without, block].sort((a, b) => a.position - b.position);
      return { canvasBlocks: { ...s.canvasBlocks, [channelId]: updated } };
    }),
  removeCanvasBlock: (channelId, blockId) =>
    set((s) => ({
      canvasBlocks: {
        ...s.canvasBlocks,
        [channelId]: (s.canvasBlocks[channelId] ?? []).filter((b) => b.id !== blockId),
      },
    })),
  loadCanvasBlocks: async (channelId) => {
    try {
      const blocks = await invoke<CanvasBlock[]>("get_canvas", { channelId });
      set((s) => ({ canvasBlocks: { ...s.canvasBlocks, [channelId]: blocks } }));
    } catch (e) {
      console.error("loadCanvasBlocks error", e);
    }
  },

  // ─── Federation ───────────────────────────────────────────────────────
  federationStatus: null,
  loadFederationStatus: async () => {
    try {
      const data = await invoke<FederationStatus>("get_federation_status", {});
      set({ federationStatus: data });
    } catch (e) {
      console.error("loadFederationStatus error", e);
    }
  },

  federationIdentity: null,
  loadFederationIdentity: async () => {
    try {
      const data = await invoke<FederationIdentity>("get_federation_identity", {});
      set({ federationIdentity: data });
    } catch (e) {
      console.error("loadFederationIdentity error", e);
    }
  },

  federatedPeers: [],
  loadFederatedPeers: async () => {
    try {
      const data = await invoke<{ peers: FederatedPeer[] }>("list_peers", {});
      set({ federatedPeers: data.peers ?? [] });
    } catch (e) {
      console.error("loadFederatedPeers error", e);
    }
  },

  peeringRequests: [],
  loadPeeringRequests: async (status?) => {
    try {
      const data = await invoke<{ requests: PeeringRequest[] }>(
        "list_peering_requests", { status }
      );
      set({ peeringRequests: data.requests ?? [] });
    } catch (e) {
      console.error("loadPeeringRequests error", e);
    }
  },

  // ─── v1.5 Collaboration & Productivity ─────────────────────────────────

  channelTasks: {},
  setChannelTasks: (channelId, tasks) =>
    set((s) => ({ channelTasks: { ...s.channelTasks, [channelId]: tasks } })),
  upsertTask: (channelId, task) =>
    set((s) => {
      const existing = s.channelTasks[channelId] ?? [];
      const without = existing.filter((t) => t.id !== task.id);
      return { channelTasks: { ...s.channelTasks, [channelId]: [...without, task].sort((a, b) => a.position - b.position) } };
    }),
  removeTask: (channelId, taskId) =>
    set((s) => ({
      channelTasks: {
        ...s.channelTasks,
        [channelId]: (s.channelTasks[channelId] ?? []).filter((t) => t.id !== taskId),
      },
    })),
  loadChannelTasks: async (channelId, serverId) => {
    try {
      const tasks = await invoke<Task[]>("list_tasks", { channelId, serverId });
      set((s) => ({ channelTasks: { ...s.channelTasks, [channelId]: tasks } }));
    } catch (e) {
      console.error("loadChannelTasks error", e);
    }
  },

  calendarEvents: {},
  setCalendarEvents: (serverId, events) =>
    set((s) => ({ calendarEvents: { ...s.calendarEvents, [serverId]: events } })),
  upsertCalendarEvent: (serverId, event) =>
    set((s) => {
      const existing = s.calendarEvents[serverId] ?? [];
      const without = existing.filter((e) => e.id !== event.id);
      return { calendarEvents: { ...s.calendarEvents, [serverId]: [...without, event] } };
    }),
  removeCalendarEvent: (serverId, eventId) =>
    set((s) => ({
      calendarEvents: {
        ...s.calendarEvents,
        [serverId]: (s.calendarEvents[serverId] ?? []).filter((e) => e.id !== eventId),
      },
    })),
  loadCalendarEvents: async (serverId, from?, to?) => {
    try {
      const events = await invoke<CalendarEvent[]>("list_calendar_events", { serverId, from, to });
      set((s) => ({ calendarEvents: { ...s.calendarEvents, [serverId]: events } }));
    } catch (e) {
      console.error("loadCalendarEvents error", e);
    }
  },

  fileVersions: {},
  setFileVersions: (attachmentId, versions) =>
    set((s) => ({ fileVersions: { ...s.fileVersions, [attachmentId]: versions } })),
  loadFileVersions: async (attachmentId) => {
    try {
      const versions = await invoke<FileVersion[]>("list_file_versions", { attachmentId });
      set((s) => ({ fileVersions: { ...s.fileVersions, [attachmentId]: versions } }));
    } catch (e) {
      console.error("loadFileVersions error", e);
    }
  },

  aiPreferences: null,
  setAiPreferences: (prefs) => set({ aiPreferences: prefs }),
  loadAiPreferences: async () => {
    try {
      const prefs = await invoke<AiPreferences>("get_ai_preferences", {});
      set({ aiPreferences: prefs });
    } catch (e) {
      console.error("loadAiPreferences error", e);
    }
  },

  // ─── v1.6 Multimedia & Expression ───────────────────────────────────────

  channelVoiceNotes: {},
  loadVoiceNotes: async (channelId) => {
    try {
      const notes = await invoke<VoiceNote[]>("list_voice_notes", { channelId });
      set((s) => ({ channelVoiceNotes: { ...s.channelVoiceNotes, [channelId]: notes } }));
    } catch (e) {
      console.error("loadVoiceNotes error", e);
    }
  },

  channelVideoNotes: {},
  loadVideoNotes: async (channelId) => {
    try {
      const notes = await invoke<VideoNote[]>("list_video_notes", { channelId });
      set((s) => ({ channelVideoNotes: { ...s.channelVideoNotes, [channelId]: notes } }));
    } catch (e) {
      console.error("loadVideoNotes error", e);
    }
  },

  storyFeed: [],
  loadStoryFeed: async () => {
    try {
      const stories = await invoke<Story[]>("get_story_feed", {});
      set({ storyFeed: stories });
    } catch (e) {
      console.error("loadStoryFeed error", e);
    }
  },

  channelDrawings: {},
  loadDrawings: async (channelId) => {
    try {
      const drawings = await invoke<Drawing[]>("list_drawings", { channelId });
      set((s) => ({ channelDrawings: { ...s.channelDrawings, [channelId]: drawings } }));
    } catch (e) {
      console.error("loadDrawings error", e);
    }
  },

  voiceSettings: null,
  setVoiceSettings: (settings) => set({ voiceSettings: settings }),
  loadVoiceSettings: async () => {
    try {
      const settings = await invoke<VoiceSettings>("get_voice_settings", {});
      set({ voiceSettings: settings });
    } catch (e) {
      console.error("loadVoiceSettings error", e);
    }
  },

  musicQueue: {},
  loadMusicQueue: async (channelId) => {
    try {
      const queue = await invoke<VoiceMusicQueueItem[]>("list_music_queue", { channelId });
      set((s) => ({ musicQueue: { ...s.musicQueue, [channelId]: queue } }));
    } catch (e) {
      console.error("loadMusicQueue error", e);
    }
  },

  mediaGalleryFilters: [],
  loadMediaGalleryFilters: async () => {
    try {
      const filters = await invoke<MediaGalleryFilter[]>("list_media_filters", {});
      set({ mediaGalleryFilters: filters });
    } catch (e) {
      console.error("loadMediaGalleryFilters error", e);
    }
  },

  // ── Accessibility ─────────────────────────────────────────────────────────
  accessibilitySettings: null,
  loadAccessibilitySettings: async () => {
    try {
      const settings = await invoke<UserAccessibilitySettings>("get_accessibility_settings", {});
      set({ accessibilitySettings: settings });
    } catch (e) {
      console.error("loadAccessibilitySettings error", e);
    }
  },
  setAccessibilitySettings: (settings) => set({ accessibilitySettings: settings }),

  voiceCaptions: {},
  loadVoiceCaptions: async (channelId) => {
    try {
      const captions = await invoke<VoiceCaption[]>("list_voice_captions", { channelId });
      set((s) => ({
        voiceCaptions: { ...s.voiceCaptions, [channelId]: captions },
      }));
    } catch (e) {
      console.error("loadVoiceCaptions error", e);
    }
  },
  addVoiceCaption: (caption) =>
    set((s) => {
      const existing = s.voiceCaptions[caption.channelId] ?? [];
      return {
        voiceCaptions: {
          ...s.voiceCaptions,
          [caption.channelId]: [caption, ...existing].slice(0, 100),
        },
      };
    }),

  // ─── v1.8 Ecosystem & Onboarding ─────────────────────────────────────

  importJobs: {},
  loadImportJobs: async (serverId) => {
    try {
      const jobs = await invoke<ImportJob[]>("list_import_jobs", { serverId });
      set((s) => ({ importJobs: { ...s.importJobs, [serverId]: jobs } }));
    } catch (e) {
      console.error("loadImportJobs error", e);
    }
  },

  onboardingProgress: null,
  loadOnboardingProgress: async () => {
    try {
      const progress = await invoke<OnboardingProgress>("get_onboarding_progress", {});
      set({ onboardingProgress: progress });
    } catch (e) {
      console.error("loadOnboardingProgress error", e);
    }
  },
  dismissOnboarding: async () => {
    try {
      const progress = await invoke<OnboardingProgress>("update_onboarding_progress", {
        dismissed: true,
      });
      set({ onboardingProgress: progress });
    } catch (e) {
      console.error("dismissOnboarding error", e);
    }
  },

  serverTemplates: [],
  loadServerTemplates: async (category) => {
    try {
      const templates = await invoke<ServerTemplate[]>("list_server_templates", {
        category: category ?? null,
      });
      set({ serverTemplates: templates });
    } catch (e) {
      console.error("loadServerTemplates error", e);
    }
  },

  analyticsSnapshots: {},
  loadAnalyticsSnapshots: async (serverId, days) => {
    try {
      const snaps = await invoke<AnalyticsSnapshot[]>("list_analytics_snapshots", {
        serverId,
        days: days ?? 30,
      });
      set((s) => ({ analyticsSnapshots: { ...s.analyticsSnapshots, [serverId]: snaps } }));
    } catch (e) {
      console.error("loadAnalyticsSnapshots error", e);
    }
  },

  marketplacePlugins: [],
  loadMarketplacePlugins: async (query, category) => {
    try {
      const plugins = await invoke<MarketplacePlugin[]>("search_marketplace_plugins", {
        q: query ?? null,
        category: category ?? null,
      });
      set({ marketplacePlugins: plugins });
    } catch (e) {
      console.error("loadMarketplacePlugins error", e);
    }
  },

  pluginReviews: {},
  loadPluginReviews: async (pluginId) => {
    try {
      const reviews = await invoke<PluginReview[]>("list_plugin_reviews", { pluginId });
      set((s) => ({ pluginReviews: { ...s.pluginReviews, [pluginId]: reviews } }));
    } catch (e) {
      console.error("loadPluginReviews error", e);
    }
  },

  serverPluginInstalls: {},
  loadServerPluginInstalls: async (serverId) => {
    try {
      const installs = await invoke<PluginInstall[]>("list_server_plugin_installs", { serverId });
      set((s) => ({ serverPluginInstalls: { ...s.serverPluginInstalls, [serverId]: installs } }));
    } catch (e) {
      console.error("loadServerPluginInstalls error", e);
    }
  },
}));

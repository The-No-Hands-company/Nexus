// ── Nexus Mobile — Shared Types ───────────────────────────────────────────────

export interface Session {
  accessToken: string;
  refreshToken: string;
  username: string;
  userId: string;
  avatar: string | null;
  serverUrl: string;
}

export interface NxServer {
  id: string;
  name: string;
  icon: string | null;
  description: string | null;
  owner_id: string;
  member_count: number;
}

export interface NxChannel {
  id: string;
  name: string | null;
  kind: "text" | "voice" | "announcement" | "dm" | "group_dm";
  server_id: string | null;
  topic: string | null;
  is_e2ee: boolean;
  position: number;
  recipients?: DmRecipient[];
}

export interface DmRecipient {
  id: string;
  username: string;
  avatar: string | null;
}

export interface NxMessage {
  id: string;
  content: string;
  author_id: string;
  author_username: string | null;
  author_avatar: string | null;
  channel_id: string;
  created_at: string;
  edited_at: string | null;
  edited: boolean;
  pinned: boolean;
  attachments: Attachment[];
  reactions: Reaction[];
  reference?: {
    message_id: string;
    channel_id: string;
  } | null;
}

export interface Attachment {
  id: string;
  filename: string;
  content_type: string;
  size: number;
  url: string | null;
  width?: number | null;
  height?: number | null;
}

export interface Reaction {
  emoji: string;
  count: number;
  me: boolean;
}

export interface NxUser {
  id: string;
  username: string;
  display_name: string | null;
  avatar: string | null;
  presence: "online" | "idle" | "dnd" | "invisible" | "offline";
  status: string | null;
}

export type GatewayStatus = "offline" | "connecting" | "online";

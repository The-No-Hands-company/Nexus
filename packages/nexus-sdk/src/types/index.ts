// ============================================================================
// Bot / Application types
// ============================================================================

export interface BotApplication {
  id: string;
  owner_id: string;
  name: string;
  description: string | null;
  avatar: string | null;
  /** SHA-256 hex hash of the raw bot token — never returned in full. */
  token_hash: string;
  public_key: string;
  is_public: boolean;
  redirect_uris: string[];
  interactions_endpoint_url: string | null;
  created_at: string;
  updated_at: string;
}

export interface BotServerInstall {
  bot_id: string;
  server_id: string;
  installed_by: string;
  installed_at: string;
  permissions: number;
  scopes: string[];
}

/** Returned once on bot creation — never retrievable again. */
export interface BotToken {
  /** Full token string: `"Bot <raw_token>"` */
  token: string;
}

// ============================================================================
// Slash command types
// ============================================================================

export const CommandType = {
  ChatInput: 1,
  User: 2,
  Message: 3,
} as const;

export type CommandType = (typeof CommandType)[keyof typeof CommandType];

export interface CommandOption {
  option_type: number;
  name: string;
  description: string;
  required: boolean;
  choices: CommandChoice[];
  options: CommandOption[];
  min_value: number | null;
  max_value: number | null;
  autocomplete: boolean;
}

export interface CommandChoice {
  name: string;
  value: string | number;
}

export interface SlashCommand {
  id: string;
  application_id: string;
  server_id: string | null;
  name: string;
  name_localizations: Record<string, string> | null;
  description: string;
  description_localizations: Record<string, string> | null;
  options: CommandOption[];
  default_member_permissions: string | null;
  dm_permission: boolean;
  command_type: number;
  version: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Interaction types
// ============================================================================

export const InteractionType = {
  ApplicationCommand: "APPLICATION_COMMAND",
  MessageComponent: "MESSAGE_COMPONENT",
  ModalSubmit: "MODAL_SUBMIT",
} as const;

export type InteractionType =
  (typeof InteractionType)[keyof typeof InteractionType];

export interface Interaction {
  id: string;
  application_id: string;
  interaction_type: string;
  data: InteractionData | null;
  server_id: string | null;
  channel_id: string | null;
  user_id: string;
  token: string;
  status: string;
  created_at: string;
  expires_at: string | null;
}

export interface InteractionData {
  command_id?: string;
  command_name?: string;
  options?: InteractionOption[];
  [key: string]: unknown;
}

export interface InteractionOption {
  name: string;
  option_type: number;
  value: string | number | boolean | null;
  options: InteractionOption[];
  focused: boolean;
}

// ============================================================================
// Webhook types
// ============================================================================

export const WebhookType = {
  Incoming: "incoming",
  Outgoing: "outgoing",
} as const;

export type WebhookType = (typeof WebhookType)[keyof typeof WebhookType];

export interface Webhook {
  id: string;
  webhook_type: string;
  server_id: string | null;
  channel_id: string | null;
  creator_id: string | null;
  name: string;
  avatar: string | null;
  /** Only present for incoming webhooks; hidden unless you own it. */
  token: string | null;
  url: string | null;
  events: string[];
  active: boolean;
  delivery_count: number;
  created_at: string;
  updated_at: string;
}

export interface Embed {
  title?: string;
  description?: string;
  url?: string;
  color?: number;
  timestamp?: string;
  footer?: { text: string; icon_url?: string };
  image?: { url: string };
  thumbnail?: { url: string };
  author?: { name: string; url?: string; icon_url?: string };
  fields?: { name: string; value: string; inline?: boolean }[];
}

// ============================================================================
// Gateway event types
// ============================================================================

export interface GatewayEventPayload {
  op: number;
  t: string | null;
  d: unknown;
  s: number | null;
}

/** Known gateway event names. */
export const GatewayEvents = {
  Ready: "READY",
  MessageCreate: "MESSAGE_CREATE",
  MessageUpdate: "MESSAGE_UPDATE",
  MessageDelete: "MESSAGE_DELETE",
  InteractionCreate: "INTERACTION_CREATE",
  ApplicationCommandCreate: "APPLICATION_COMMAND_CREATE",
  ApplicationCommandUpdate: "APPLICATION_COMMAND_UPDATE",
  ApplicationCommandDelete: "APPLICATION_COMMAND_DELETE",
  WebhookDelivery: "WEBHOOK_DELIVERY",
} as const;

export type GatewayEventName =
  (typeof GatewayEvents)[keyof typeof GatewayEvents];

import type {
  BotApplication,
  BotToken,
  BotServerInstall,
  SlashCommand,
  Webhook,
  Interaction,
  Embed,
  CommandOption,
} from "../types/index.js";

// ============================================================================
// Core HTTP client
// ============================================================================

export class NexusAPIError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: unknown,
    message?: string
  ) {
    super(message ?? `Nexus API error ${status}`);
    this.name = "NexusAPIError";
  }
}

export class RestClient {
  private readonly baseUrl: string;
  private readonly token: string; // "Bot <raw>"

  constructor(token: string, baseUrl = "http://localhost:3000/api/v1") {
    this.token = token.startsWith("Bot ") ? token : `Bot ${token}`;
    this.baseUrl = baseUrl.replace(/\/$/, "");
  }

  // --------------------------------------------------------------------------
  // Generic request helper
  // --------------------------------------------------------------------------

  async request<T = unknown>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      Authorization: this.token,
      "Content-Type": "application/json",
      "User-Agent": "@nexus/sdk/0.7.0",
    };

    const res = await fetch(url, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    if (res.status === 204) {
      return undefined as T;
    }

    const json = await res.json().catch(() => null);

    if (!res.ok) {
      throw new NexusAPIError(res.status, json);
    }

    return json as T;
  }

  // Convenience helpers
  get<T>(path: string) {
    return this.request<T>("GET", path);
  }
  post<T>(path: string, body?: unknown) {
    return this.request<T>("POST", path, body);
  }
  patch<T>(path: string, body?: unknown) {
    return this.request<T>("PATCH", path, body);
  }
  put<T>(path: string, body?: unknown) {
    return this.request<T>("PUT", path, body);
  }
  delete<T = void>(path: string) {
    return this.request<T>("DELETE", path);
  }

  // --------------------------------------------------------------------------
  // Applications (bot management)
  // --------------------------------------------------------------------------

  /** List all bot applications owned by the authenticated user. */
  listApplications(): Promise<BotApplication[]> {
    return this.get("/applications");
  }

  /** Get a single bot application. */
  getApplication(appId: string): Promise<BotApplication> {
    return this.get(`/applications/${appId}`);
  }

  /** Create a new bot application. Returns the app + one-time plain token. */
  createApplication(
    name: string,
    opts?: {
      description?: string;
      is_public?: boolean;
      redirect_uris?: string[];
      interactions_endpoint_url?: string;
    }
  ): Promise<[BotApplication, BotToken]> {
    return this.post("/applications", { name, ...opts });
  }

  /** Update a bot application. */
  updateApplication(
    appId: string,
    data: Partial<{
      name: string;
      description: string;
      avatar: string;
      is_public: boolean;
      redirect_uris: string[];
      interactions_endpoint_url: string;
    }>
  ): Promise<BotApplication> {
    return this.patch(`/applications/${appId}`, data);
  }

  /** Delete a bot application. */
  deleteApplication(appId: string): Promise<void> {
    return this.delete(`/applications/${appId}`);
  }

  /** Reset (rotate) the bot token. Returns the new plain token once. */
  resetToken(appId: string): Promise<BotToken> {
    return this.post(`/applications/${appId}/token/reset`);
  }

  // --------------------------------------------------------------------------
  // Server bot integrations
  // --------------------------------------------------------------------------

  /** List all bots installed in a server. */
  listServerBots(serverId: string): Promise<BotServerInstall[]> {
    return this.get(`/servers/${serverId}/integrations`);
  }

  /** Install a bot into a server. */
  installBot(
    serverId: string,
    botId: string,
    opts?: { permissions?: number; scopes?: string[] }
  ): Promise<BotServerInstall> {
    return this.post(`/servers/${serverId}/integrations`, {
      bot_id: botId,
      ...opts,
    });
  }

  /** Remove a bot from a server. */
  uninstallBot(serverId: string, botId: string): Promise<void> {
    return this.delete(`/servers/${serverId}/integrations/${botId}`);
  }

  // --------------------------------------------------------------------------
  // Global slash commands
  // --------------------------------------------------------------------------

  /** List all global commands for an application. */
  getGlobalCommands(appId: string): Promise<SlashCommand[]> {
    return this.get(`/applications/${appId}/commands`);
  }

  /** Get a single global command. */
  getGlobalCommand(appId: string, commandId: string): Promise<SlashCommand> {
    return this.get(`/applications/${appId}/commands/${commandId}`);
  }

  /** Create a global command. */
  createGlobalCommand(
    appId: string,
    data: {
      name: string;
      description: string;
      options?: CommandOption[];
      command_type?: number;
      default_member_permissions?: string;
      dm_permission?: boolean;
    }
  ): Promise<SlashCommand> {
    return this.post(`/applications/${appId}/commands`, data);
  }

  /** Edit an existing global command. */
  editGlobalCommand(
    appId: string,
    commandId: string,
    data: Partial<{
      name: string;
      description: string;
      options: CommandOption[];
      command_type: number;
      default_member_permissions: string;
      dm_permission: boolean;
    }>
  ): Promise<SlashCommand> {
    return this.patch(`/applications/${appId}/commands/${commandId}`, data);
  }

  /** Delete a global command. */
  deleteGlobalCommand(appId: string, commandId: string): Promise<void> {
    return this.delete(`/applications/${appId}/commands/${commandId}`);
  }

  /**
   * Bulk overwrite **all** global commands for an application.
   * Any command not in the array will be deleted.
   */
  bulkOverwriteGlobalCommands(
    appId: string,
    commands: Array<{
      name: string;
      description: string;
      options?: CommandOption[];
      command_type?: number;
      default_member_permissions?: string;
      dm_permission?: boolean;
    }>
  ): Promise<SlashCommand[]> {
    return this.put(`/applications/${appId}/commands`, commands);
  }

  // --------------------------------------------------------------------------
  // Server-scoped slash commands
  // --------------------------------------------------------------------------

  /** List all commands for this application in a specific server. */
  getServerCommands(
    appId: string,
    serverId: string
  ): Promise<SlashCommand[]> {
    return this.get(
      `/applications/${appId}/guilds/${serverId}/commands`
    );
  }

  /** Create a server-scoped command. */
  createServerCommand(
    appId: string,
    serverId: string,
    data: {
      name: string;
      description: string;
      options?: CommandOption[];
      command_type?: number;
    }
  ): Promise<SlashCommand> {
    return this.post(
      `/applications/${appId}/guilds/${serverId}/commands`,
      data
    );
  }

  /** Bulk overwrite all commands for an application in a specific server. */
  bulkOverwriteServerCommands(
    appId: string,
    serverId: string,
    commands: Array<{
      name: string;
      description: string;
      options?: CommandOption[];
      command_type?: number;
    }>
  ): Promise<SlashCommand[]> {
    return this.put(
      `/applications/${appId}/guilds/${serverId}/commands`,
      commands
    );
  }

  /** Delete a server-scoped command. */
  deleteServerCommand(
    appId: string,
    serverId: string,
    commandId: string
  ): Promise<void> {
    return this.delete(
      `/applications/${appId}/guilds/${serverId}/commands/${commandId}`
    );
  }

  // --------------------------------------------------------------------------
  // Interactions (responding to commands)
  // --------------------------------------------------------------------------

  /**
   * Respond to an interaction.
   * `response_type` values:
   *  - 4 = CHANNEL_MESSAGE_WITH_SOURCE
   *  - 5 = DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE
   *  - 7 = UPDATE_MESSAGE
   */
  createInteractionResponse(
    interactionId: string,
    data: {
      response_type: number;
      data?: {
        content?: string;
        embeds?: Embed[];
        ephemeral?: boolean;
        tts?: boolean;
      };
    }
  ): Promise<void> {
    return this.post(`/interactions/${interactionId}/callback`, data);
  }

  // --------------------------------------------------------------------------
  // Webhooks
  // --------------------------------------------------------------------------

  /** List all webhooks in a channel. */
  getChannelWebhooks(channelId: string): Promise<Webhook[]> {
    return this.get(`/channels/${channelId}/webhooks`);
  }

  /** Create an incoming webhook in a channel. */
  createWebhook(
    channelId: string,
    name: string,
    avatar?: string
  ): Promise<Webhook> {
    return this.post(`/channels/${channelId}/webhooks`, { name, avatar });
  }

  /** Get a webhook by ID (owner only — returns token). */
  getWebhook(webhookId: string): Promise<Webhook> {
    return this.get(`/webhooks/${webhookId}`);
  }

  /** Modify a webhook. */
  modifyWebhook(
    webhookId: string,
    data: Partial<{
      name: string;
      avatar: string;
      channel_id: string;
      url: string;
      events: string[];
      active: boolean;
    }>
  ): Promise<Webhook> {
    return this.patch(`/webhooks/${webhookId}`, data);
  }

  /** Delete a webhook. */
  deleteWebhook(webhookId: string): Promise<void> {
    return this.delete(`/webhooks/${webhookId}`);
  }

  /**
   * Execute a webhook (send a message via webhook URL).
   * Does **not** require bot auth — uses the token in the URL path.
   */
  async executeWebhook(
    webhookId: string,
    token: string,
    data: {
      content?: string;
      username?: string;
      avatar_url?: string;
      embeds?: Embed[];
    }
  ): Promise<void> {
    const url = `${this.baseUrl}/webhooks/${webhookId}/${token}`;
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    if (!res.ok) {
      const body = await res.json().catch(() => null);
      throw new NexusAPIError(res.status, body);
    }
  }
}

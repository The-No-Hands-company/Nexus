import type { BotApplication, BotToken, BotServerInstall, SlashCommand, Webhook, Embed, CommandOption } from "../types/index.js";
export declare class NexusAPIError extends Error {
    readonly status: number;
    readonly body: unknown;
    constructor(status: number, body: unknown, message?: string);
}
export declare class RestClient {
    private readonly baseUrl;
    private readonly token;
    constructor(token: string, baseUrl?: string);
    request<T = unknown>(method: string, path: string, body?: unknown): Promise<T>;
    get<T>(path: string): Promise<T>;
    post<T>(path: string, body?: unknown): Promise<T>;
    patch<T>(path: string, body?: unknown): Promise<T>;
    put<T>(path: string, body?: unknown): Promise<T>;
    delete<T = void>(path: string): Promise<T>;
    /** List all bot applications owned by the authenticated user. */
    listApplications(): Promise<BotApplication[]>;
    /** Get a single bot application. */
    getApplication(appId: string): Promise<BotApplication>;
    /** Create a new bot application. Returns the app + one-time plain token. */
    createApplication(name: string, opts?: {
        description?: string;
        is_public?: boolean;
        redirect_uris?: string[];
        interactions_endpoint_url?: string;
    }): Promise<[BotApplication, BotToken]>;
    /** Update a bot application. */
    updateApplication(appId: string, data: Partial<{
        name: string;
        description: string;
        avatar: string;
        is_public: boolean;
        redirect_uris: string[];
        interactions_endpoint_url: string;
    }>): Promise<BotApplication>;
    /** Delete a bot application. */
    deleteApplication(appId: string): Promise<void>;
    /** Reset (rotate) the bot token. Returns the new plain token once. */
    resetToken(appId: string): Promise<BotToken>;
    /** List all bots installed in a server. */
    listServerBots(serverId: string): Promise<BotServerInstall[]>;
    /** Install a bot into a server. */
    installBot(serverId: string, botId: string, opts?: {
        permissions?: number;
        scopes?: string[];
    }): Promise<BotServerInstall>;
    /** Remove a bot from a server. */
    uninstallBot(serverId: string, botId: string): Promise<void>;
    /** List all global commands for an application. */
    getGlobalCommands(appId: string): Promise<SlashCommand[]>;
    /** Get a single global command. */
    getGlobalCommand(appId: string, commandId: string): Promise<SlashCommand>;
    /** Create a global command. */
    createGlobalCommand(appId: string, data: {
        name: string;
        description: string;
        options?: CommandOption[];
        command_type?: number;
        default_member_permissions?: string;
        dm_permission?: boolean;
    }): Promise<SlashCommand>;
    /** Edit an existing global command. */
    editGlobalCommand(appId: string, commandId: string, data: Partial<{
        name: string;
        description: string;
        options: CommandOption[];
        command_type: number;
        default_member_permissions: string;
        dm_permission: boolean;
    }>): Promise<SlashCommand>;
    /** Delete a global command. */
    deleteGlobalCommand(appId: string, commandId: string): Promise<void>;
    /**
     * Bulk overwrite **all** global commands for an application.
     * Any command not in the array will be deleted.
     */
    bulkOverwriteGlobalCommands(appId: string, commands: Array<{
        name: string;
        description: string;
        options?: CommandOption[];
        command_type?: number;
        default_member_permissions?: string;
        dm_permission?: boolean;
    }>): Promise<SlashCommand[]>;
    /** List all commands for this application in a specific server. */
    getServerCommands(appId: string, serverId: string): Promise<SlashCommand[]>;
    /** Create a server-scoped command. */
    createServerCommand(appId: string, serverId: string, data: {
        name: string;
        description: string;
        options?: CommandOption[];
        command_type?: number;
    }): Promise<SlashCommand>;
    /** Bulk overwrite all commands for an application in a specific server. */
    bulkOverwriteServerCommands(appId: string, serverId: string, commands: Array<{
        name: string;
        description: string;
        options?: CommandOption[];
        command_type?: number;
    }>): Promise<SlashCommand[]>;
    /** Delete a server-scoped command. */
    deleteServerCommand(appId: string, serverId: string, commandId: string): Promise<void>;
    /**
     * Respond to an interaction.
     * `response_type` values:
     *  - 4 = CHANNEL_MESSAGE_WITH_SOURCE
     *  - 5 = DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE
     *  - 7 = UPDATE_MESSAGE
     */
    createInteractionResponse(interactionId: string, data: {
        response_type: number;
        data?: {
            content?: string;
            embeds?: Embed[];
            ephemeral?: boolean;
            tts?: boolean;
        };
    }): Promise<void>;
    /** List all webhooks in a channel. */
    getChannelWebhooks(channelId: string): Promise<Webhook[]>;
    /** Create an incoming webhook in a channel. */
    createWebhook(channelId: string, name: string, avatar?: string): Promise<Webhook>;
    /** Get a webhook by ID (owner only — returns token). */
    getWebhook(webhookId: string): Promise<Webhook>;
    /** Modify a webhook. */
    modifyWebhook(webhookId: string, data: Partial<{
        name: string;
        avatar: string;
        channel_id: string;
        url: string;
        events: string[];
        active: boolean;
    }>): Promise<Webhook>;
    /** Delete a webhook. */
    deleteWebhook(webhookId: string): Promise<void>;
    /**
     * Execute a webhook (send a message via webhook URL).
     * Does **not** require bot auth — uses the token in the URL path.
     */
    executeWebhook(webhookId: string, token: string, data: {
        content?: string;
        username?: string;
        avatar_url?: string;
        embeds?: Embed[];
    }): Promise<void>;
}
//# sourceMappingURL=RestClient.d.ts.map
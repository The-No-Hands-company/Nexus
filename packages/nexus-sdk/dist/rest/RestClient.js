"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.RestClient = exports.NexusAPIError = void 0;
// ============================================================================
// Core HTTP client
// ============================================================================
class NexusAPIError extends Error {
    status;
    body;
    constructor(status, body, message) {
        super(message ?? `Nexus API error ${status}`);
        this.status = status;
        this.body = body;
        this.name = "NexusAPIError";
    }
}
exports.NexusAPIError = NexusAPIError;
class RestClient {
    baseUrl;
    token; // "Bot <raw>"
    constructor(token, baseUrl = "http://localhost:3000/api/v1") {
        this.token = token.startsWith("Bot ") ? token : `Bot ${token}`;
        this.baseUrl = baseUrl.replace(/\/$/, "");
    }
    // --------------------------------------------------------------------------
    // Generic request helper
    // --------------------------------------------------------------------------
    async request(method, path, body) {
        const url = `${this.baseUrl}${path}`;
        const headers = {
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
            return undefined;
        }
        const json = await res.json().catch(() => null);
        if (!res.ok) {
            throw new NexusAPIError(res.status, json);
        }
        return json;
    }
    // Convenience helpers
    get(path) {
        return this.request("GET", path);
    }
    post(path, body) {
        return this.request("POST", path, body);
    }
    patch(path, body) {
        return this.request("PATCH", path, body);
    }
    put(path, body) {
        return this.request("PUT", path, body);
    }
    delete(path) {
        return this.request("DELETE", path);
    }
    // --------------------------------------------------------------------------
    // Applications (bot management)
    // --------------------------------------------------------------------------
    /** List all bot applications owned by the authenticated user. */
    listApplications() {
        return this.get("/applications");
    }
    /** Get a single bot application. */
    getApplication(appId) {
        return this.get(`/applications/${appId}`);
    }
    /** Create a new bot application. Returns the app + one-time plain token. */
    createApplication(name, opts) {
        return this.post("/applications", { name, ...opts });
    }
    /** Update a bot application. */
    updateApplication(appId, data) {
        return this.patch(`/applications/${appId}`, data);
    }
    /** Delete a bot application. */
    deleteApplication(appId) {
        return this.delete(`/applications/${appId}`);
    }
    /** Reset (rotate) the bot token. Returns the new plain token once. */
    resetToken(appId) {
        return this.post(`/applications/${appId}/token/reset`);
    }
    // --------------------------------------------------------------------------
    // Server bot integrations
    // --------------------------------------------------------------------------
    /** List all bots installed in a server. */
    listServerBots(serverId) {
        return this.get(`/servers/${serverId}/integrations`);
    }
    /** Install a bot into a server. */
    installBot(serverId, botId, opts) {
        return this.post(`/servers/${serverId}/integrations`, {
            bot_id: botId,
            ...opts,
        });
    }
    /** Remove a bot from a server. */
    uninstallBot(serverId, botId) {
        return this.delete(`/servers/${serverId}/integrations/${botId}`);
    }
    // --------------------------------------------------------------------------
    // Global slash commands
    // --------------------------------------------------------------------------
    /** List all global commands for an application. */
    getGlobalCommands(appId) {
        return this.get(`/applications/${appId}/commands`);
    }
    /** Get a single global command. */
    getGlobalCommand(appId, commandId) {
        return this.get(`/applications/${appId}/commands/${commandId}`);
    }
    /** Create a global command. */
    createGlobalCommand(appId, data) {
        return this.post(`/applications/${appId}/commands`, data);
    }
    /** Edit an existing global command. */
    editGlobalCommand(appId, commandId, data) {
        return this.patch(`/applications/${appId}/commands/${commandId}`, data);
    }
    /** Delete a global command. */
    deleteGlobalCommand(appId, commandId) {
        return this.delete(`/applications/${appId}/commands/${commandId}`);
    }
    /**
     * Bulk overwrite **all** global commands for an application.
     * Any command not in the array will be deleted.
     */
    bulkOverwriteGlobalCommands(appId, commands) {
        return this.put(`/applications/${appId}/commands`, commands);
    }
    // --------------------------------------------------------------------------
    // Server-scoped slash commands
    // --------------------------------------------------------------------------
    /** List all commands for this application in a specific server. */
    getServerCommands(appId, serverId) {
        return this.get(`/applications/${appId}/guilds/${serverId}/commands`);
    }
    /** Create a server-scoped command. */
    createServerCommand(appId, serverId, data) {
        return this.post(`/applications/${appId}/guilds/${serverId}/commands`, data);
    }
    /** Bulk overwrite all commands for an application in a specific server. */
    bulkOverwriteServerCommands(appId, serverId, commands) {
        return this.put(`/applications/${appId}/guilds/${serverId}/commands`, commands);
    }
    /** Delete a server-scoped command. */
    deleteServerCommand(appId, serverId, commandId) {
        return this.delete(`/applications/${appId}/guilds/${serverId}/commands/${commandId}`);
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
    createInteractionResponse(interactionId, data) {
        return this.post(`/interactions/${interactionId}/callback`, data);
    }
    // --------------------------------------------------------------------------
    // Webhooks
    // --------------------------------------------------------------------------
    /** List all webhooks in a channel. */
    getChannelWebhooks(channelId) {
        return this.get(`/channels/${channelId}/webhooks`);
    }
    /** Create an incoming webhook in a channel. */
    createWebhook(channelId, name, avatar) {
        return this.post(`/channels/${channelId}/webhooks`, { name, avatar });
    }
    /** Get a webhook by ID (owner only — returns token). */
    getWebhook(webhookId) {
        return this.get(`/webhooks/${webhookId}`);
    }
    /** Modify a webhook. */
    modifyWebhook(webhookId, data) {
        return this.patch(`/webhooks/${webhookId}`, data);
    }
    /** Delete a webhook. */
    deleteWebhook(webhookId) {
        return this.delete(`/webhooks/${webhookId}`);
    }
    /**
     * Execute a webhook (send a message via webhook URL).
     * Does **not** require bot auth — uses the token in the URL path.
     */
    async executeWebhook(webhookId, token, data) {
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
exports.RestClient = RestClient;
//# sourceMappingURL=RestClient.js.map
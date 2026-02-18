"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.NexusClient = void 0;
const events_1 = require("events");
const RestClient_js_1 = require("./rest/RestClient.js");
const GatewayClient_js_1 = require("./gateway/GatewayClient.js");
const SlashCommandBuilder_js_1 = require("./builders/SlashCommandBuilder.js");
/**
 * The main Nexus bot client.
 *
 * ### Quick start
 * ```ts
 * import { NexusClient, SlashCommandBuilder } from "@nexus/sdk";
 *
 * const client = new NexusClient({ token: process.env.BOT_TOKEN! });
 *
 * client.command(
 *   new SlashCommandBuilder()
 *     .setName("ping")
 *     .setDescription("Replies with Pong!"),
 *   async (interaction) => {
 *     await client.reply(interaction, { content: "Pong! 🏓" });
 *   },
 * );
 *
 * await client.login("your-app-id");
 * ```
 */
class NexusClient extends events_1.EventEmitter {
    /** Low-level REST API client — use to call any endpoint directly. */
    rest;
    gateway = null;
    gatewayUrl;
    token;
    /** Registered commands: name → { definition, handler }. */
    _commands = new Map();
    constructor(opts) {
        super();
        this.token = opts.token;
        this.rest = new RestClient_js_1.RestClient(opts.token, opts.restUrl);
        this.gatewayUrl = opts.gatewayUrl ?? "ws://localhost:3001";
    }
    // --------------------------------------------------------------------------
    // Command registration
    // --------------------------------------------------------------------------
    /**
     * Register a slash command and its handler.
     * All registered commands are bulk-pushed to the API during {@link login}.
     *
     * ```ts
     * client.command(
     *   new SlashCommandBuilder().setName("ping").setDescription("Pong!"),
     *   async (interaction) => client.reply(interaction, { content: "Pong!" }),
     * );
     * ```
     */
    command(builder, execute) {
        const def = builder instanceof SlashCommandBuilder_js_1.SlashCommandBuilder ? builder.build() : builder;
        this._commands.set(def.name, { def, execute });
        return this;
    }
    // --------------------------------------------------------------------------
    // Login / disconnect
    // --------------------------------------------------------------------------
    /**
     * Register all commands with the API, then open the gateway connection.
     * Resolves once the gateway emits `ready`.
     *
     * @param appId  Your bot application ID (Nexus snowflake string).
     */
    async login(appId) {
        if (this._commands.size > 0) {
            const defs = [...this._commands.values()].map((c) => c.def);
            await this.rest.bulkOverwriteGlobalCommands(appId, defs);
        }
        this.gateway = new GatewayClient_js_1.GatewayClient({
            gatewayUrl: this.gatewayUrl,
            token: this.token,
        });
        this.gateway.on("ready", (d) => this.emit("ready", d));
        this.gateway.on("dispatch", (e, d) => this.emit("dispatch", e, d));
        this.gateway.on("close", (c, r) => this.emit("close", c, r));
        this.gateway.on("error", (e) => this.emit("error", e));
        this.gateway.on("reconnecting", (a) => this.emit("reconnecting", a));
        this.gateway.on("interaction", (interaction) => {
            this.emit("interaction", interaction);
            void this._route(interaction);
        });
        await this.gateway.connect();
    }
    /** Disconnect and stop reconnecting. */
    destroy() {
        this.gateway?.destroy();
        this.gateway = null;
    }
    // --------------------------------------------------------------------------
    // Interaction helpers
    // --------------------------------------------------------------------------
    /**
     * Send a reply to an interaction.
     *
     * ```ts
     * await client.reply(interaction, { content: "Hello!", ephemeral: true });
     * ```
     */
    reply(interaction, data) {
        const { ephemeral, ...rest } = data;
        return this.rest.createInteractionResponse(interaction.id, {
            response_type: 4, // CHANNEL_MESSAGE_WITH_SOURCE
            data: { ...rest, ...(ephemeral ? { flags: 64 } : {}) },
        });
    }
    /**
     * Acknowledge the interaction without sending content yet.
     * Follow up via `rest.createInteractionResponse` with type 7.
     */
    deferReply(interaction, ephemeral = false) {
        return this.rest.createInteractionResponse(interaction.id, {
            response_type: 5, // DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE
            data: ephemeral ? { ephemeral: true } : undefined,
        });
    }
    on(event, listener) {
        return super.on(event, listener);
    }
    once(event, listener) {
        return super.once(event, listener);
    }
    emit(event, ...args) {
        return super.emit(event, ...args);
    }
    // --------------------------------------------------------------------------
    // Private
    // --------------------------------------------------------------------------
    async _route(interaction) {
        const data = interaction.data;
        const name = data?.command_name ?? data?.name;
        if (!name)
            return;
        const entry = this._commands.get(name);
        if (!entry)
            return;
        try {
            await entry.execute(interaction);
        }
        catch (err) {
            this.emit("error", err instanceof Error ? err : new Error(String(err)));
        }
    }
}
exports.NexusClient = NexusClient;
//# sourceMappingURL=NexusClient.js.map
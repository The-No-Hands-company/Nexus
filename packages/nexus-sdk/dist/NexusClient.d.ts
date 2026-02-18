import { EventEmitter } from "events";
import { RestClient } from "./rest/RestClient.js";
import { type GatewayClientEvents } from "./gateway/GatewayClient.js";
import type { Interaction, Embed } from "./types/index.js";
import { SlashCommandBuilder } from "./builders/SlashCommandBuilder.js";
export interface NexusClientOptions {
    /** Bot token: `"Bot <raw>"` or just the raw token string. */
    token: string;
    /** Base URL of the Nexus REST API (default: `http://localhost:3000/api/v1`). */
    restUrl?: string;
    /** WebSocket URL of the Nexus gateway (default: `ws://localhost:3001`). */
    gatewayUrl?: string;
}
type CommandDefinition = ReturnType<SlashCommandBuilder["build"]>;
type CommandExecute = (interaction: Interaction) => Promise<void> | void;
export type NexusClientEvents = GatewayClientEvents;
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
export declare class NexusClient extends EventEmitter {
    /** Low-level REST API client — use to call any endpoint directly. */
    readonly rest: RestClient;
    private gateway;
    private readonly gatewayUrl;
    private readonly token;
    /** Registered commands: name → { definition, handler }. */
    private readonly _commands;
    constructor(opts: NexusClientOptions);
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
    command(builder: SlashCommandBuilder | CommandDefinition, execute: CommandExecute): this;
    /**
     * Register all commands with the API, then open the gateway connection.
     * Resolves once the gateway emits `ready`.
     *
     * @param appId  Your bot application ID (Nexus snowflake string).
     */
    login(appId: string): Promise<void>;
    /** Disconnect and stop reconnecting. */
    destroy(): void;
    /**
     * Send a reply to an interaction.
     *
     * ```ts
     * await client.reply(interaction, { content: "Hello!", ephemeral: true });
     * ```
     */
    reply(interaction: Interaction, data: {
        content?: string;
        embeds?: Embed[];
        ephemeral?: boolean;
        tts?: boolean;
    }): Promise<void>;
    /**
     * Acknowledge the interaction without sending content yet.
     * Follow up via `rest.createInteractionResponse` with type 7.
     */
    deferReply(interaction: Interaction, ephemeral?: boolean): Promise<void>;
    on<K extends keyof NexusClientEvents>(event: K, listener: (...args: NexusClientEvents[K]) => void): this;
    on(event: string | symbol, listener: (...args: unknown[]) => void): this;
    once<K extends keyof NexusClientEvents>(event: K, listener: (...args: NexusClientEvents[K]) => void): this;
    once(event: string | symbol, listener: (...args: unknown[]) => void): this;
    emit<K extends keyof NexusClientEvents>(event: K, ...args: NexusClientEvents[K]): boolean;
    emit(event: string | symbol, ...args: unknown[]): boolean;
    private _route;
}
export {};
//# sourceMappingURL=NexusClient.d.ts.map
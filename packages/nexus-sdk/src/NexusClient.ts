import { EventEmitter } from "events";
import { RestClient } from "./rest/RestClient.js";
import {
  GatewayClient,
  type GatewayClientEvents,
} from "./gateway/GatewayClient.js";
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
export class NexusClient extends EventEmitter {
  /** Low-level REST API client — use to call any endpoint directly. */
  readonly rest: RestClient;

  private gateway: GatewayClient | null = null;
  private readonly gatewayUrl: string;
  private readonly token: string;

  /** Registered commands: name → { definition, handler }. */
  private readonly _commands = new Map<
    string,
    { def: CommandDefinition; execute: CommandExecute }
  >();

  constructor(opts: NexusClientOptions) {
    super();
    this.token = opts.token;
    this.rest = new RestClient(opts.token, opts.restUrl);
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
  command(
    builder: SlashCommandBuilder | CommandDefinition,
    execute: CommandExecute
  ): this {
    const def =
      builder instanceof SlashCommandBuilder ? builder.build() : builder;
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
  async login(appId: string): Promise<void> {
    if (this._commands.size > 0) {
      const defs = [...this._commands.values()].map((c) => c.def);
      await this.rest.bulkOverwriteGlobalCommands(appId, defs);
    }

    this.gateway = new GatewayClient({
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
  destroy(): void {
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
  reply(
    interaction: Interaction,
    data: { content?: string; embeds?: Embed[]; ephemeral?: boolean; tts?: boolean }
  ): Promise<void> {
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
  deferReply(interaction: Interaction, ephemeral = false): Promise<void> {
    return this.rest.createInteractionResponse(interaction.id, {
      response_type: 5, // DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE
      data: ephemeral ? { ephemeral: true } : undefined,
    });
  }

  // --------------------------------------------------------------------------
  // Typed EventEmitter overloads
  // --------------------------------------------------------------------------

  override on<K extends keyof NexusClientEvents>(
    event: K,
    listener: (...args: NexusClientEvents[K]) => void
  ): this;
  override on(event: string | symbol, listener: (...args: unknown[]) => void): this;
  override on(event: string | symbol, listener: (...args: unknown[]) => void): this {
    return super.on(event, listener);
  }

  override once<K extends keyof NexusClientEvents>(
    event: K,
    listener: (...args: NexusClientEvents[K]) => void
  ): this;
  override once(event: string | symbol, listener: (...args: unknown[]) => void): this;
  override once(event: string | symbol, listener: (...args: unknown[]) => void): this {
    return super.once(event, listener);
  }

  override emit<K extends keyof NexusClientEvents>(
    event: K,
    ...args: NexusClientEvents[K]
  ): boolean;
  override emit(event: string | symbol, ...args: unknown[]): boolean;
  override emit(event: string | symbol, ...args: unknown[]): boolean {
    return super.emit(event, ...args);
  }

  // --------------------------------------------------------------------------
  // Private
  // --------------------------------------------------------------------------

  private async _route(interaction: Interaction): Promise<void> {
    const data = interaction.data as
      | { command_name?: string; name?: string }
      | null
      | undefined;
    const name = data?.command_name ?? data?.name;
    if (!name) return;

    const entry = this._commands.get(name);
    if (!entry) return;

    try {
      await entry.execute(interaction);
    } catch (err) {
      this.emit(
        "error",
        err instanceof Error ? err : new Error(String(err))
      );
    }
  }
}







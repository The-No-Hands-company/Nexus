// Main client
export { NexusClient } from "./NexusClient.js";
export type { NexusClientOptions, NexusClientEvents } from "./NexusClient.js";

// REST
export { RestClient, NexusAPIError } from "./rest/RestClient.js";

// Gateway
export { GatewayClient, GatewayOp } from "./gateway/GatewayClient.js";
export type {
  GatewayClientOptions,
  GatewayClientEvents,
} from "./gateway/GatewayClient.js";

// Builders
export {
  SlashCommandBuilder,
  SlashCommandOptionBuilder,
  OptionType,
} from "./builders/SlashCommandBuilder.js";
export { EmbedBuilder } from "./builders/EmbedBuilder.js";

// Types
export * from "./types/index.js";

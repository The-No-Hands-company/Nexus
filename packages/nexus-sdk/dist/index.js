"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.EmbedBuilder = exports.OptionType = exports.SlashCommandOptionBuilder = exports.SlashCommandBuilder = exports.GatewayOp = exports.GatewayClient = exports.NexusAPIError = exports.RestClient = exports.NexusClient = void 0;
// Main client
var NexusClient_js_1 = require("./NexusClient.js");
Object.defineProperty(exports, "NexusClient", { enumerable: true, get: function () { return NexusClient_js_1.NexusClient; } });
// REST
var RestClient_js_1 = require("./rest/RestClient.js");
Object.defineProperty(exports, "RestClient", { enumerable: true, get: function () { return RestClient_js_1.RestClient; } });
Object.defineProperty(exports, "NexusAPIError", { enumerable: true, get: function () { return RestClient_js_1.NexusAPIError; } });
// Gateway
var GatewayClient_js_1 = require("./gateway/GatewayClient.js");
Object.defineProperty(exports, "GatewayClient", { enumerable: true, get: function () { return GatewayClient_js_1.GatewayClient; } });
Object.defineProperty(exports, "GatewayOp", { enumerable: true, get: function () { return GatewayClient_js_1.GatewayOp; } });
// Builders
var SlashCommandBuilder_js_1 = require("./builders/SlashCommandBuilder.js");
Object.defineProperty(exports, "SlashCommandBuilder", { enumerable: true, get: function () { return SlashCommandBuilder_js_1.SlashCommandBuilder; } });
Object.defineProperty(exports, "SlashCommandOptionBuilder", { enumerable: true, get: function () { return SlashCommandBuilder_js_1.SlashCommandOptionBuilder; } });
Object.defineProperty(exports, "OptionType", { enumerable: true, get: function () { return SlashCommandBuilder_js_1.OptionType; } });
var EmbedBuilder_js_1 = require("./builders/EmbedBuilder.js");
Object.defineProperty(exports, "EmbedBuilder", { enumerable: true, get: function () { return EmbedBuilder_js_1.EmbedBuilder; } });
// Types
__exportStar(require("./types/index.js"), exports);
//# sourceMappingURL=index.js.map
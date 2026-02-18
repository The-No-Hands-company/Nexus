"use strict";
// ============================================================================
// Bot / Application types
// ============================================================================
Object.defineProperty(exports, "__esModule", { value: true });
exports.GatewayEvents = exports.WebhookType = exports.InteractionType = exports.CommandType = void 0;
// ============================================================================
// Slash command types
// ============================================================================
exports.CommandType = {
    ChatInput: 1,
    User: 2,
    Message: 3,
};
// ============================================================================
// Interaction types
// ============================================================================
exports.InteractionType = {
    ApplicationCommand: "APPLICATION_COMMAND",
    MessageComponent: "MESSAGE_COMPONENT",
    ModalSubmit: "MODAL_SUBMIT",
};
// ============================================================================
// Webhook types
// ============================================================================
exports.WebhookType = {
    Incoming: "incoming",
    Outgoing: "outgoing",
};
/** Known gateway event names. */
exports.GatewayEvents = {
    Ready: "READY",
    MessageCreate: "MESSAGE_CREATE",
    MessageUpdate: "MESSAGE_UPDATE",
    MessageDelete: "MESSAGE_DELETE",
    InteractionCreate: "INTERACTION_CREATE",
    ApplicationCommandCreate: "APPLICATION_COMMAND_CREATE",
    ApplicationCommandUpdate: "APPLICATION_COMMAND_UPDATE",
    ApplicationCommandDelete: "APPLICATION_COMMAND_DELETE",
    WebhookDelivery: "WEBHOOK_DELIVERY",
};
//# sourceMappingURL=index.js.map
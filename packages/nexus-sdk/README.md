# @nexus/sdk

TypeScript client library for building bots on the **Nexus** platform.

## Requirements

- Node.js ≥ 18
- A running Nexus server (REST API + gateway)

## Installation

```bash
npm install @nexus/sdk
```

## Quick start

```ts
import { NexusClient, SlashCommandBuilder, EmbedBuilder } from "@nexus/sdk";

const client = new NexusClient({ token: process.env.BOT_TOKEN! });

client.command(
  new SlashCommandBuilder()
    .setName("ping")
    .setDescription("Replies with Pong!"),
  async (interaction) => {
    await client.reply(interaction, { content: "Pong! 🏓" });
  },
);

client.command(
  new SlashCommandBuilder()
    .setName("info")
    .setDescription("Show bot info"),
  async (interaction) => {
    const embed = new EmbedBuilder()
      .setTitle("Bot Info")
      .setDescription("Running on Nexus 🚀")
      .setColor(0x5865f2)
      .build();

    await client.reply(interaction, { embeds: [embed] });
  },
);

client.on("ready", () => console.log("Bot is ready!"));
client.on("error", (err) => console.error("Error:", err));

// Replace "your-app-id" with your bot's application ID from the Nexus dashboard.
await client.login("your-app-id");
```

## Configuration

| Option | Default | Description |
|---|---|---|
| `token` | *(required)* | Bot token — `"Bot <raw>"` or just the raw string. |
| `restUrl` | `http://localhost:3000/api/v1` | Base URL of the Nexus REST API. |
| `gatewayUrl` | `ws://localhost:3001` | WebSocket URL of the Nexus gateway. |

## API Reference

### `NexusClient`

The main entry point.

```ts
const client = new NexusClient({ token, restUrl?, gatewayUrl? });
```

#### Methods

| Method | Description |
|---|---|
| `command(builder, handler)` | Register a slash command + handler. Returns `this` for chaining. |
| `login(appId)` | Bulk-register commands, then open the gateway. Returns `Promise<void>`. |
| `destroy()` | Close the gateway connection. |
| `reply(interaction, data)` | Send a message reply to an interaction. |
| `deferReply(interaction, ephemeral?)` | Acknowledge an interaction without an immediate response. |

#### Events

```ts
client.on("ready", (data) => { /* gateway READY payload */ });
client.on("dispatch", (eventName, data) => { /* any gateway event */ });
client.on("interaction", (interaction) => { /* raw INTERACTION_CREATE */ });
client.on("close", (code, reason) => { /* gateway closed */ });
client.on("error", (error) => { /* error */ });
client.on("reconnecting", (attempt) => { /* attempting reconnect #attempt */ });
```

### `RestClient`

Low-level HTTP client exposed as `client.rest`. Use it to call any API endpoint directly.

```ts
// Example: list global commands
const commands = await client.rest.getGlobalCommands("app-id");

// Example: execute a webhook
await client.rest.executeWebhook("webhook-id", "webhook-token", {
  content: "Hello from a webhook!",
});
```

### `SlashCommandBuilder`

```ts
new SlashCommandBuilder()
  .setName("roll")
  .setDescription("Roll a dice")
  .addIntegerOption((opt) =>
    opt.setName("sides").setDescription("Number of sides").setRequired(true),
  )
  .build();
```

Supported option adders: `addStringOption`, `addIntegerOption`, `addBooleanOption`,
`addUserOption`, `addChannelOption`, `addRoleOption`, `addNumberOption`, `addSubCommand`.

### `EmbedBuilder`

```ts
new EmbedBuilder()
  .setTitle("Hello")
  .setDescription("World")
  .setColor(0x00aaff)
  .setTimestamp()
  .addField("Field 1", "Value 1", true)
  .build();
```

### `GatewayClient`

Use directly if you need lower-level gateway control (e.g. without the full `NexusClient`).

```ts
import { GatewayClient } from "@nexus/sdk";

const gw = new GatewayClient({ token: "...", gatewayUrl: "ws://..." });
gw.on("ready", (d) => console.log("Ready", d));
gw.on("dispatch", (event, data) => console.log(event, data));
await gw.connect();
```

## Building from source

```bash
npm install
npm run build          # compile to dist/
npm run dev            # watch mode
```

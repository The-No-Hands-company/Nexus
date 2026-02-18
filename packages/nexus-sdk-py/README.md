# nexus-sdk (Python)

Async Python client library for building bots on the **Nexus** platform.

## Requirements

- Python ≥ 3.10
- A running Nexus server (REST API + gateway)

## Installation

```bash
pip install nexus-sdk
# or, from the repo root:
pip install packages/nexus-sdk-py
```

## Quick start

```python
import asyncio
import os
from nexus_sdk import NexusClient, SlashCommandBuilder, EmbedBuilder

client = NexusClient(token=os.environ["BOT_TOKEN"])

@client.command(
    SlashCommandBuilder()
    .set_name("ping")
    .set_description("Replies with Pong!")
)
async def ping(interaction):
    await client.reply(interaction, content="Pong! 🏓")

@client.command(
    SlashCommandBuilder()
    .set_name("info")
    .set_description("Show bot info")
)
async def info(interaction):
    embed = (
        EmbedBuilder()
        .set_title("Bot Info")
        .set_description("Running on Nexus 🚀")
        .set_color(0x7C6AF7)
        .build()
    )
    await client.reply(interaction, embeds=[embed])

@client.on("READY")
async def ready(data):
    print("Bot is ready!")

@client.on("MESSAGE_CREATE")
async def on_message(data):
    print(f"[{data['channel_id']}] {data['author_username']}: {data['content']}")

asyncio.run(client.login("your-app-id"))
```

## Configuration

| Parameter | Default | Description |
|---|---|---|
| `token` | *(required)* | Bot token string. `"Bot <raw>"` prefix added automatically. |
| `rest_url` | `http://localhost:3000/api/v1` | Nexus REST API base URL. |
| `gateway_url` | `ws://localhost:3001` | Nexus gateway WebSocket URL. |

## API Reference

### `NexusClient`

```python
client = NexusClient(token, rest_url?, gateway_url?)
```

| Method | Description |
|---|---|
| `@client.command(builder)` | Decorator to register a slash command + async handler. |
| `@client.on(event)` | Decorator to register a raw gateway event handler. |
| `await client.login(app_id)` | Register commands + connect to gateway. Blocks until `destroy()`. |
| `client.destroy()` | Disconnect the gateway. |
| `await client.reply(interaction, ...)` | Send a reply message for an interaction. |
| `await client.defer_reply(interaction, ephemeral?)` | Defer the response. |
| `client.rest` | Direct access to the `RestClient`. |

### `RestClient`

Full async REST client. Use `client.rest` or instantiate directly:

```python
async with RestClient("Bot mytoken") as rest:
    apps = await rest.list_applications()
    cmds = await rest.get_global_commands("app-id")
```

### `GatewayClient`

Low-level WebSocket client with auto-reconnect:

```python
gw = GatewayClient(token="Bot mytoken")

@gw.on("READY")
async def ready(data): print("Ready:", data)

await gw.connect()
```

### Builders

```python
# Slash command
cmd = (
    SlashCommandBuilder()
    .set_name("roll")
    .set_description("Roll a dice")
    .add_integer_option(lambda o:
        o.set_name("sides").set_description("Sides").set_required(True)
    )
    .build()
)

# Embed
embed = (
    EmbedBuilder()
    .set_title("Result")
    .set_description("You rolled a **6**!")
    .set_color(0x3ba55c)
    .set_timestamp()
    .build()
)
```

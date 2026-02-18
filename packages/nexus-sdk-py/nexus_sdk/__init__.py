"""Nexus Bot SDK for Python.

```python
import asyncio
import os
from nexus_sdk import NexusClient, SlashCommandBuilder, EmbedBuilder

client = NexusClient(token=os.environ["BOT_TOKEN"])

@client.command(
    SlashCommandBuilder().set_name("ping").set_description("Replies with Pong!")
)
async def ping(interaction):
    await client.reply(interaction, content="Pong! 🏓")

asyncio.run(client.login("your-app-id"))
```
"""

from .builders import EmbedBuilder, OptionType, SlashCommandBuilder, SlashCommandOptionBuilder
from .client import NexusClient
from .gateway import GatewayClient, GatewayOp
from .rest import NexusAPIError, RestClient
from .types import (
    BotApplication,
    BotServerInstall,
    BotToken,
    CommandChoice,
    CommandOption,
    CommandType,
    Embed,
    EmbedAuthor,
    EmbedField,
    EmbedFooter,
    EmbedImage,
    Interaction,
    InteractionData,
    InteractionOption,
    InteractionType,
    SlashCommand,
    Webhook,
    WebhookType,
)

__version__ = "0.7.0"
__all__ = [
    "NexusClient",
    "RestClient",
    "NexusAPIError",
    "GatewayClient",
    "GatewayOp",
    "SlashCommandBuilder",
    "SlashCommandOptionBuilder",
    "OptionType",
    "EmbedBuilder",
    # Types
    "BotApplication",
    "BotToken",
    "BotServerInstall",
    "CommandType",
    "CommandChoice",
    "CommandOption",
    "SlashCommand",
    "InteractionType",
    "Interaction",
    "InteractionData",
    "InteractionOption",
    "WebhookType",
    "Webhook",
    "Embed",
    "EmbedFooter",
    "EmbedImage",
    "EmbedAuthor",
    "EmbedField",
]

"""Domain types matching the Nexus Rust API models (snake_case)."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


# ── Bot application ──────────────────────────────────────────────────────────

@dataclass
class BotApplication:
    id: str
    name: str
    description: str
    owner_id: str
    avatar: str | None = None


@dataclass
class BotToken:
    token: str
    bot_user_id: str


@dataclass
class BotServerInstall:
    server_id: str
    bot_user_id: str
    application_id: str
    permissions: int


# ── Slash commands ────────────────────────────────────────────────────────────

class CommandType:
    CHAT_INPUT = 1
    USER = 2
    MESSAGE = 3


@dataclass
class CommandChoice:
    name: str
    value: str | int | float


@dataclass
class CommandOption:
    type: int
    name: str
    description: str
    required: bool = False
    choices: list[CommandChoice] = field(default_factory=list)
    options: list[CommandOption] = field(default_factory=list)
    min_value: float | None = None
    max_value: float | None = None
    min_length: int | None = None
    max_length: int | None = None


@dataclass
class SlashCommand:
    id: str
    application_id: str
    name: str
    description: str
    type: int = CommandType.CHAT_INPUT
    options: list[CommandOption] = field(default_factory=list)
    default_member_permissions: str | None = None
    dm_permission: bool = True
    guild_id: str | None = None


# ── Interactions ──────────────────────────────────────────────────────────────

class InteractionType:
    PING = 1
    APPLICATION_COMMAND = 2
    MESSAGE_COMPONENT = 3
    AUTOCOMPLETE = 4
    MODAL_SUBMIT = 5


@dataclass
class InteractionOption:
    name: str
    type: int
    value: Any = None
    options: list[InteractionOption] = field(default_factory=list)


@dataclass
class InteractionData:
    id: str
    name: str
    type: int
    options: list[InteractionOption] = field(default_factory=list)
    resolved: dict[str, Any] | None = None


@dataclass
class Interaction:
    id: str
    application_id: str
    type: int
    token: str
    version: int = 1
    data: InteractionData | None = None
    guild_id: str | None = None
    channel_id: str | None = None
    user_id: str | None = None


# ── Webhooks ──────────────────────────────────────────────────────────────────

class WebhookType:
    INCOMING = 1
    CHANNEL_FOLLOWER = 2


@dataclass
class EmbedFooter:
    text: str
    icon_url: str | None = None


@dataclass
class EmbedImage:
    url: str
    height: int | None = None
    width: int | None = None


@dataclass
class EmbedAuthor:
    name: str
    url: str | None = None
    icon_url: str | None = None


@dataclass
class EmbedField:
    name: str
    value: str
    inline: bool = False


@dataclass
class Embed:
    title: str | None = None
    description: str | None = None
    url: str | None = None
    color: int | None = None
    timestamp: str | None = None
    footer: EmbedFooter | None = None
    image: EmbedImage | None = None
    thumbnail: EmbedImage | None = None
    author: EmbedAuthor | None = None
    fields: list[EmbedField] = field(default_factory=list)


@dataclass
class Webhook:
    id: str
    type: int
    server_id: str | None
    channel_id: str | None
    name: str
    token: str | None = None
    avatar: str | None = None
    application_id: str | None = None

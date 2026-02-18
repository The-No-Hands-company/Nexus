"""Fluent builders for slash commands and embeds."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Self


# ── Option types ──────────────────────────────────────────────────────────────

class OptionType:
    SUB_COMMAND = 1
    SUB_COMMAND_GROUP = 2
    STRING = 3
    INTEGER = 4
    BOOLEAN = 5
    USER = 6
    CHANNEL = 7
    ROLE = 8
    MENTIONABLE = 9
    NUMBER = 10
    ATTACHMENT = 11


# ── Slash command builder ─────────────────────────────────────────────────────

class SlashCommandOptionBuilder:
    """Fluent builder for a single slash command option."""

    def __init__(self, option_type: int) -> None:
        self._type = option_type
        self._name = ""
        self._description = ""
        self._required = False
        self._choices: list[dict[str, Any]] = []
        self._options: list[dict[str, Any]] = []
        self._min_value: float | None = None
        self._max_value: float | None = None
        self._min_length: int | None = None
        self._max_length: int | None = None

    def set_name(self, name: str) -> Self:
        self._name = name
        return self

    def set_description(self, description: str) -> Self:
        self._description = description
        return self

    def set_required(self, required: bool = True) -> Self:
        self._required = required
        return self

    def add_choice(self, name: str, value: str | int | float) -> Self:
        self._choices.append({"name": name, "value": value})
        return self

    def set_min_value(self, v: float) -> Self:
        self._min_value = v
        return self

    def set_max_value(self, v: float) -> Self:
        self._max_value = v
        return self

    def set_min_length(self, v: int) -> Self:
        self._min_length = v
        return self

    def set_max_length(self, v: int) -> Self:
        self._max_length = v
        return self

    def build(self) -> dict[str, Any]:
        d: dict[str, Any] = {
            "type": self._type,
            "name": self._name,
            "description": self._description,
            "required": self._required,
        }
        if self._choices:
            d["choices"] = self._choices
        if self._options:
            d["options"] = self._options
        if self._min_value is not None:
            d["min_value"] = self._min_value
        if self._max_value is not None:
            d["max_value"] = self._max_value
        if self._min_length is not None:
            d["min_length"] = self._min_length
        if self._max_length is not None:
            d["max_length"] = self._max_length
        return d


def _make_adder(t: int) -> str:
    return t  # type: ignore[return-value]  # only for type annotation trick


class SlashCommandBuilder:
    """Fluent builder for a slash command definition.

    ```python
    cmd = (
        SlashCommandBuilder()
        .set_name("ping")
        .set_description("Replies with Pong!")
        .build()
    )
    ```
    """

    def __init__(self) -> None:
        self._name = ""
        self._description = ""
        self._type = 1  # CHAT_INPUT
        self._options: list[dict[str, Any]] = []
        self._default_member_permissions: str | None = None
        self._dm_permission: bool = True

    def set_name(self, name: str) -> Self:
        self._name = name
        return self

    def set_description(self, description: str) -> Self:
        self._description = description
        return self

    def set_type(self, t: int) -> Self:
        self._type = t
        return self

    def set_default_member_permissions(self, perms: str | int) -> Self:
        self._default_member_permissions = str(perms)
        return self

    def set_dm_permission(self, allow: bool) -> Self:
        self._dm_permission = allow
        return self

    def _add_option(self, option_type: int, fn: Any) -> Self:
        builder = SlashCommandOptionBuilder(option_type)
        fn(builder)
        self._options.append(builder.build())
        return self

    def add_string_option(self, fn: Any) -> Self:
        return self._add_option(OptionType.STRING, fn)

    def add_integer_option(self, fn: Any) -> Self:
        return self._add_option(OptionType.INTEGER, fn)

    def add_boolean_option(self, fn: Any) -> Self:
        return self._add_option(OptionType.BOOLEAN, fn)

    def add_user_option(self, fn: Any) -> Self:
        return self._add_option(OptionType.USER, fn)

    def add_channel_option(self, fn: Any) -> Self:
        return self._add_option(OptionType.CHANNEL, fn)

    def add_role_option(self, fn: Any) -> Self:
        return self._add_option(OptionType.ROLE, fn)

    def add_number_option(self, fn: Any) -> Self:
        return self._add_option(OptionType.NUMBER, fn)

    def add_sub_command(self, fn: Any) -> Self:
        return self._add_option(OptionType.SUB_COMMAND, fn)

    def build(self) -> dict[str, Any]:
        d: dict[str, Any] = {
            "name": self._name,
            "description": self._description,
            "type": self._type,
            "dm_permission": self._dm_permission,
        }
        if self._options:
            d["options"] = self._options
        if self._default_member_permissions is not None:
            d["default_member_permissions"] = self._default_member_permissions
        return d

    def to_json(self) -> str:
        import json as _json
        return _json.dumps(self.build())


# ── Embed builder ─────────────────────────────────────────────────────────────

class EmbedBuilder:
    """Fluent builder for message embeds.

    ```python
    embed = (
        EmbedBuilder()
        .set_title("Hello")
        .set_description("World")
        .set_color(0x7c6af7)
        .build()
    )
    ```
    """

    def __init__(self) -> None:
        self._data: dict[str, Any] = {}

    def set_title(self, title: str) -> Self:
        self._data["title"] = title
        return self

    def set_description(self, description: str) -> Self:
        self._data["description"] = description
        return self

    def set_url(self, url: str) -> Self:
        self._data["url"] = url
        return self

    def set_color(self, color: int) -> Self:
        self._data["color"] = color
        return self

    def set_timestamp(self, ts: str | None = None) -> Self:
        from datetime import datetime, timezone
        self._data["timestamp"] = ts or datetime.now(timezone.utc).isoformat()
        return self

    def set_footer(self, text: str, icon_url: str | None = None) -> Self:
        self._data["footer"] = {"text": text, **({"icon_url": icon_url} if icon_url else {})}
        return self

    def set_image(self, url: str) -> Self:
        self._data["image"] = {"url": url}
        return self

    def set_thumbnail(self, url: str) -> Self:
        self._data["thumbnail"] = {"url": url}
        return self

    def set_author(self, name: str, url: str | None = None, icon_url: str | None = None) -> Self:
        a: dict[str, Any] = {"name": name}
        if url:
            a["url"] = url
        if icon_url:
            a["icon_url"] = icon_url
        self._data["author"] = a
        return self

    def add_field(self, name: str, value: str, inline: bool = False) -> Self:
        self._data.setdefault("fields", []).append({"name": name, "value": value, "inline": inline})
        return self

    def add_fields(self, *fields: tuple[str, str, bool]) -> Self:
        for name, value, inline in fields:
            self.add_field(name, value, inline)
        return self

    def build(self) -> dict[str, Any]:
        return dict(self._data)

    def to_json(self) -> str:
        import json as _json
        return _json.dumps(self.build())

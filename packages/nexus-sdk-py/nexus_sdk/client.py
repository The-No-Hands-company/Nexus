"""High-level NexusClient combining REST + gateway."""

from __future__ import annotations

import asyncio
import logging
from collections.abc import Callable, Coroutine
from typing import Any

from .builders import SlashCommandBuilder
from .gateway import GatewayClient, GatewayOp
from .rest import RestClient
from .types import Embed, Interaction

log = logging.getLogger(__name__)

CommandHandler = Callable[[Interaction], Coroutine[Any, Any, None] | None]


class NexusClient:
    """The main Nexus bot client.

    ### Quick start

    ```python
    import asyncio
    import os
    from nexus_sdk import NexusClient, SlashCommandBuilder

    client = NexusClient(token=os.environ["BOT_TOKEN"])

    @client.command(
        SlashCommandBuilder().set_name("ping").set_description("Pong!")
    )
    async def ping(interaction):
        await client.reply(interaction, content="Pong! 🏓")

    asyncio.run(client.login("your-app-id"))
    ```
    """

    def __init__(
        self,
        token: str,
        rest_url: str = "http://localhost:3000/api/v1",
        gateway_url: str = "ws://localhost:3001",
    ) -> None:
        self.rest = RestClient(token, rest_url)
        self._gateway = GatewayClient(token, gateway_url)
        self._commands: dict[str, tuple[dict[str, Any], CommandHandler]] = {}
        self._app_id: str | None = None

        # Forward gateway events to user-registered listeners
        self._gateway.add_listener("INTERACTION_CREATE", self._route)  # type: ignore[arg-type]

    # ── Command registration ──────────────────────────────────────────────────

    def command(
        self,
        builder: SlashCommandBuilder | dict[str, Any],
    ) -> Callable[[CommandHandler], CommandHandler]:
        """Decorator that registers a slash command handler.

        ```python
        @client.command(SlashCommandBuilder().set_name("ping").set_description("Pong!"))
        async def ping(interaction): ...
        ```
        """
        def decorator(fn: CommandHandler) -> CommandHandler:
            defn = builder.build() if isinstance(builder, SlashCommandBuilder) else builder
            self._commands[defn["name"]] = (defn, fn)
            return fn
        return decorator

    def on(self, event: str) -> Callable[[Any], Any]:
        """Delegate to the underlying gateway's ``on`` decorator."""
        return self._gateway.on(event)

    # ── Login ─────────────────────────────────────────────────────────────────

    async def login(self, app_id: str) -> None:
        """Register all commands with the API then open the gateway.

        Blocks until :meth:`destroy` is called.
        """
        self._app_id = app_id
        if self._commands:
            defs = [defn for defn, _ in self._commands.values()]
            await self.rest.bulk_overwrite_global_commands(app_id, defs)

        await self._gateway.connect()

    def destroy(self) -> None:
        """Disconnect the gateway."""
        self._gateway.destroy()

    # ── Interaction helpers ───────────────────────────────────────────────────

    async def reply(
        self,
        interaction: Interaction | dict[str, Any],
        *,
        content: str | None = None,
        embeds: list[Embed | dict[str, Any]] | None = None,
        ephemeral: bool = False,
        tts: bool = False,
    ) -> None:
        """Send a reply to an interaction (response type 4)."""
        interaction_id = (
            interaction.id if isinstance(interaction, Interaction)
            else interaction["id"]
        )
        data: dict[str, Any] = {}
        if content is not None:
            data["content"] = content
        if embeds:
            from .rest import _to_dict
            data["embeds"] = [_to_dict(e) if not isinstance(e, dict) else e for e in embeds]
        if ephemeral:
            data["flags"] = 64
        if tts:
            data["tts"] = True
        await self.rest.create_interaction_response(interaction_id, 4, data)

    async def defer_reply(
        self,
        interaction: Interaction | dict[str, Any],
        ephemeral: bool = False,
    ) -> None:
        """Acknowledge the interaction without an immediate response (type 5)."""
        interaction_id = (
            interaction.id if isinstance(interaction, Interaction)
            else interaction["id"]
        )
        data = {"ephemeral": True} if ephemeral else None
        await self.rest.create_interaction_response(interaction_id, 5, data)

    # ── Private ───────────────────────────────────────────────────────────────

    async def _route(self, data: Any) -> None:
        if not isinstance(data, dict):
            return
        cmd_data = data.get("data") or {}
        name = cmd_data.get("name") or cmd_data.get("command_name")
        if not name:
            return
        entry = self._commands.get(name)
        if not entry:
            return
        _, handler = entry
        try:
            result = handler(data)  # type: ignore[arg-type]
            if asyncio.iscoroutine(result):
                await result
        except Exception as exc:
            log.error("Command handler %r raised: %s", name, exc, exc_info=True)

"""Async REST client for the Nexus API."""

from __future__ import annotations

import dataclasses
import json
from typing import Any, TypeVar

import httpx

from .types import (
    BotApplication,
    BotToken,
    BotServerInstall,
    Embed,
    Interaction,
    SlashCommand,
    Webhook,
)

T = TypeVar("T")

_BASE = "http://localhost:3000/api/v1"


class NexusAPIError(Exception):
    """Raised when the Nexus API returns a non-2xx response."""

    def __init__(self, status: int, message: str) -> None:
        super().__init__(f"HTTP {status}: {message}")
        self.status = status
        self.message = message


def _to_dict(obj: Any) -> Any:
    """Recursively convert dataclasses to dicts, dropping None values."""
    if dataclasses.is_dataclass(obj) and not isinstance(obj, type):
        return {
            k: _to_dict(v)
            for k, v in dataclasses.asdict(obj).items()  # type: ignore[arg-type]
            if v is not None
        }
    if isinstance(obj, list):
        return [_to_dict(i) for i in obj]
    return obj


class RestClient:
    """Low-level async Nexus REST client.

    Uses an underlying :class:`httpx.AsyncClient` that is managed
    automatically; call :meth:`close` when done (or use as an async
    context manager).

    ```python
    async with RestClient("Bot mytoken") as rest:
        apps = await rest.list_applications()
    ```
    """

    def __init__(self, token: str, base_url: str = _BASE) -> None:
        if not token.startswith("Bot "):
            token = f"Bot {token}"
        self._client = httpx.AsyncClient(
            base_url=base_url,
            headers={"Authorization": token, "Content-Type": "application/json"},
            timeout=30.0,
        )

    async def close(self) -> None:
        await self._client.aclose()

    async def __aenter__(self) -> "RestClient":
        return self

    async def __aexit__(self, *_: Any) -> None:
        await self.close()

    # ── Internal ──────────────────────────────────────────────────────────────

    async def _request(self, method: str, path: str, **kwargs: Any) -> Any:
        if "json" in kwargs and kwargs["json"] is not None:
            kwargs["json"] = _to_dict(kwargs["json"])
        resp = await self._client.request(method, path, **kwargs)
        if not resp.is_success:
            try:
                msg = resp.json().get("error", resp.text)
            except Exception:
                msg = resp.text
            raise NexusAPIError(resp.status_code, msg)
        if resp.status_code == 204 or not resp.content:
            return None
        return resp.json()

    async def _get(self, path: str, **kw: Any) -> Any:
        return await self._request("GET", path, **kw)

    async def _post(self, path: str, body: Any = None, **kw: Any) -> Any:
        return await self._request("POST", path, json=body, **kw)

    async def _patch(self, path: str, body: Any = None) -> Any:
        return await self._request("PATCH", path, json=body)

    async def _put(self, path: str, body: Any = None) -> Any:
        return await self._request("PUT", path, json=body)

    async def _delete(self, path: str) -> None:
        await self._request("DELETE", path)

    # ── Applications ──────────────────────────────────────────────────────────

    async def list_applications(self) -> list[dict[str, Any]]:
        return await self._get("/applications")

    async def get_application(self, app_id: str) -> dict[str, Any]:
        return await self._get(f"/applications/{app_id}")

    async def create_application(self, name: str, description: str = "") -> dict[str, Any]:
        return await self._post("/applications", {"name": name, "description": description})

    async def update_application(self, app_id: str, **fields: Any) -> dict[str, Any]:
        return await self._patch(f"/applications/{app_id}", fields)

    async def delete_application(self, app_id: str) -> None:
        await self._delete(f"/applications/{app_id}")

    async def reset_token(self, app_id: str) -> dict[str, Any]:
        return await self._post(f"/applications/{app_id}/reset-token")

    # ── Server installs ───────────────────────────────────────────────────────

    async def list_server_bots(self, server_id: str) -> list[dict[str, Any]]:
        return await self._get(f"/servers/{server_id}/bots")

    async def install_bot(self, server_id: str, app_id: str, permissions: int = 0) -> dict[str, Any]:
        return await self._post(f"/servers/{server_id}/bots", {
            "application_id": app_id, "permissions": permissions
        })

    async def uninstall_bot(self, server_id: str, bot_user_id: str) -> None:
        await self._delete(f"/servers/{server_id}/bots/{bot_user_id}")

    # ── Global commands ───────────────────────────────────────────────────────

    async def get_global_commands(self, app_id: str) -> list[dict[str, Any]]:
        return await self._get(f"/applications/{app_id}/commands")

    async def get_global_command(self, app_id: str, cmd_id: str) -> dict[str, Any]:
        return await self._get(f"/applications/{app_id}/commands/{cmd_id}")

    async def create_global_command(self, app_id: str, data: dict[str, Any]) -> dict[str, Any]:
        return await self._post(f"/applications/{app_id}/commands", data)

    async def edit_global_command(self, app_id: str, cmd_id: str, data: dict[str, Any]) -> dict[str, Any]:
        return await self._patch(f"/applications/{app_id}/commands/{cmd_id}", data)

    async def delete_global_command(self, app_id: str, cmd_id: str) -> None:
        await self._delete(f"/applications/{app_id}/commands/{cmd_id}")

    async def bulk_overwrite_global_commands(
        self, app_id: str, commands: list[dict[str, Any]]
    ) -> list[dict[str, Any]]:
        return await self._put(f"/applications/{app_id}/commands", commands)

    # ── Server commands ───────────────────────────────────────────────────────

    async def get_server_commands(self, app_id: str, server_id: str) -> list[dict[str, Any]]:
        return await self._get(f"/applications/{app_id}/guilds/{server_id}/commands")

    async def create_server_command(
        self, app_id: str, server_id: str, data: dict[str, Any]
    ) -> dict[str, Any]:
        return await self._post(f"/applications/{app_id}/guilds/{server_id}/commands", data)

    async def bulk_overwrite_server_commands(
        self, app_id: str, server_id: str, commands: list[dict[str, Any]]
    ) -> list[dict[str, Any]]:
        return await self._put(f"/applications/{app_id}/guilds/{server_id}/commands", commands)

    async def delete_server_command(self, app_id: str, server_id: str, cmd_id: str) -> None:
        await self._delete(f"/applications/{app_id}/guilds/{server_id}/commands/{cmd_id}")

    # ── Interactions ──────────────────────────────────────────────────────────

    async def create_interaction_response(
        self,
        interaction_id: str,
        response_type: int,
        data: dict[str, Any] | None = None,
    ) -> None:
        body: dict[str, Any] = {"type": response_type}
        if data:
            body["data"] = data
        await self._post(f"/interactions/{interaction_id}/callback", body)

    # ── Webhooks ──────────────────────────────────────────────────────────────

    async def get_channel_webhooks(self, channel_id: str) -> list[dict[str, Any]]:
        return await self._get(f"/channels/{channel_id}/webhooks")

    async def create_webhook(self, channel_id: str, name: str, avatar: str | None = None) -> dict[str, Any]:
        return await self._post(f"/channels/{channel_id}/webhooks", {
            "name": name, **({"avatar": avatar} if avatar else {})
        })

    async def get_webhook(self, webhook_id: str) -> dict[str, Any]:
        return await self._get(f"/webhooks/{webhook_id}")

    async def modify_webhook(self, webhook_id: str, **fields: Any) -> dict[str, Any]:
        return await self._patch(f"/webhooks/{webhook_id}", fields)

    async def delete_webhook(self, webhook_id: str) -> None:
        await self._delete(f"/webhooks/{webhook_id}")

    async def execute_webhook(
        self,
        webhook_id: str,
        webhook_token: str,
        *,
        content: str | None = None,
        embeds: list[Embed] | None = None,
        username: str | None = None,
        avatar_url: str | None = None,
    ) -> None:
        body: dict[str, Any] = {}
        if content:
            body["content"] = content
        if embeds:
            body["embeds"] = [_to_dict(e) for e in embeds]
        if username:
            body["username"] = username
        if avatar_url:
            body["avatar_url"] = avatar_url
        # Webhook execution uses token in URL — no Authorization header.
        resp = await self._client.post(
            f"/webhooks/{webhook_id}/{webhook_token}",
            json=body,
            headers={"Authorization": ""},
        )
        if not resp.is_success:
            raise NexusAPIError(resp.status_code, resp.text)

"""Async WebSocket gateway client for Nexus."""

from __future__ import annotations

import asyncio
import json
import logging
from collections.abc import Callable, Coroutine
from typing import Any

import websockets
from websockets.asyncio.client import connect, ClientConnection

log = logging.getLogger(__name__)

# ── Op codes ──────────────────────────────────────────────────────────────────

class GatewayOp:
    DISPATCH = 0
    HEARTBEAT = 1
    IDENTIFY = 2
    RESUME = 6
    RECONNECT = 7
    HEARTBEAT_ACK = 11


EventHandler = Callable[..., Coroutine[Any, Any, None] | None]


class GatewayClient:
    """Low-level asyncio WebSocket gateway client.

    Emits events via ``on(event_name, handler)`` style registration.

    ```python
    gw = GatewayClient(token="Bot mytoken")

    @gw.on("READY")
    async def on_ready(data):
        print("Connected as", data["user"]["username"])

    @gw.on("INTERACTION_CREATE")
    async def on_interaction(data):
        print("Got interaction", data)

    await gw.connect()
    ```
    """

    def __init__(
        self,
        token: str,
        gateway_url: str = "ws://localhost:3001",
        heartbeat_interval: float = 30.0,
        max_reconnect_attempts: int = 10,
    ) -> None:
        if not token.startswith("Bot "):
            token = f"Bot {token}"
        self.token = token
        self.gateway_url = gateway_url
        self.heartbeat_interval = heartbeat_interval
        self.max_reconnect_attempts = max_reconnect_attempts

        self._handlers: dict[str, list[EventHandler]] = {}
        self._session_id: str | None = None
        self._seq: int | None = None
        self._ws: ClientConnection | None = None
        self._hb_task: asyncio.Task[None] | None = None
        self._running = False

    # ── Registration ──────────────────────────────────────────────────────────

    def on(self, event: str) -> Callable[[EventHandler], EventHandler]:
        """Decorator to register an event handler.

        ```python
        @client.on("READY")
        async def on_ready(data): ...
        ```
        """
        def decorator(fn: EventHandler) -> EventHandler:
            self._handlers.setdefault(event.upper(), []).append(fn)
            return fn
        return decorator

    def add_listener(self, event: str, handler: EventHandler) -> None:
        self._handlers.setdefault(event.upper(), []).append(handler)

    def remove_listener(self, event: str, handler: EventHandler) -> None:
        self._handlers.get(event.upper(), []).remove(handler)

    async def _dispatch(self, event: str, data: Any) -> None:
        for handler in self._handlers.get(event.upper(), []):
            result = handler(data)
            if asyncio.iscoroutine(result):
                await result

    # ── Connect ───────────────────────────────────────────────────────────────

    async def connect(self) -> None:
        """Open the gateway and block until :meth:`destroy` is called."""
        self._running = True
        attempt = 0
        while self._running:
            try:
                await self._run()
                attempt = 0
            except (websockets.exceptions.ConnectionClosed, OSError) as exc:
                if not self._running:
                    break
                attempt += 1
                if attempt > self.max_reconnect_attempts:
                    log.error("Gateway: max reconnect attempts reached, giving up")
                    break
                delay = min(2 ** attempt, 30)
                log.warning("Gateway: disconnected (%s), reconnecting in %ss (attempt %d)", exc, delay, attempt)
                await self._dispatch("RECONNECTING", {"attempt": attempt})
                await asyncio.sleep(delay)

    def destroy(self) -> None:
        """Stop the gateway client."""
        self._running = False
        if self._hb_task:
            self._hb_task.cancel()
        if self._ws:
            asyncio.ensure_future(self._ws.close())

    # ── Internal ──────────────────────────────────────────────────────────────

    async def _run(self) -> None:
        async with connect(self.gateway_url) as ws:
            self._ws = ws
            self._hb_task = asyncio.create_task(self._heartbeat_loop(ws))
            try:
                identify_or_resume = {
                    "op": GatewayOp.IDENTIFY,
                    "d": {"token": self.token, "properties": {"$os": "python"}},
                } if not self._session_id else {
                    "op": GatewayOp.RESUME,
                    "d": {
                        "token": self.token,
                        "session_id": self._session_id,
                        "seq": self._seq,
                    },
                }
                await ws.send(json.dumps(identify_or_resume))

                async for raw in ws:
                    payload = json.loads(raw)
                    await self._handle(payload)
            finally:
                self._hb_task.cancel()
                self._ws = None

    async def _handle(self, payload: dict[str, Any]) -> None:
        op: int = payload.get("op", -1)
        data: Any = payload.get("d")
        seq: int | None = payload.get("s")
        event: str | None = payload.get("t")

        if seq is not None:
            self._seq = seq

        if op == GatewayOp.DISPATCH:
            if event == "READY":
                self._session_id = data.get("session_id") if isinstance(data, dict) else None
                await self._dispatch("READY", data)
            if event:
                await self._dispatch(event, data)

        elif op == GatewayOp.HEARTBEAT:
            if self._ws:
                await self._ws.send(json.dumps({"op": GatewayOp.HEARTBEAT, "d": self._seq}))

        elif op == GatewayOp.RECONNECT:
            log.info("Gateway: server requested reconnect")
            if self._ws:
                await self._ws.close()

        elif op == GatewayOp.HEARTBEAT_ACK:
            pass  # heartbeat acknowledged

    async def _heartbeat_loop(self, ws: ClientConnection) -> None:
        while True:
            await asyncio.sleep(self.heartbeat_interval)
            try:
                await ws.send(json.dumps({"op": GatewayOp.HEARTBEAT, "d": self._seq}))
            except Exception:
                break

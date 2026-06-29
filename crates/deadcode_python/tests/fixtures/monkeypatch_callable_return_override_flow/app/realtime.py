from typing import Protocol

from app.redis_client import get_client


class SyncClient(Protocol):
    def publish(self, channel: str, message: str) -> None: ...


def _get_sync_client() -> SyncClient:
    return get_client()


def publish_event() -> None:
    _get_sync_client().publish("updates", "payload")

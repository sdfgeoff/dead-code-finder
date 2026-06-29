from typing import Protocol

from app.redis_client import get_client


class SyncClient(Protocol):
    def publish(self, channel: str, message: str) -> None: ...


def _get_sync_client() -> SyncClient:
    return get_client()


def publish_event() -> None:
    client = _get_sync_client()
    client.publish("updates", "payload")

import pytest
from collections.abc import Callable

from app import realtime
from app.main import run


class FakeClient:
    def __init__(self) -> None:
        self.values: dict[str, str] = {}
        self.messages: list[tuple[str, str]] = []

    def set(self, key: str, value: str) -> None:
        self.values[key] = value

    def get(self, key: str) -> str | None:
        return self.values.get(key)

    def publish(self, channel: str, message: str) -> None:
        self.messages.append((channel, message))

    def unused(self) -> None:
        pass


@pytest.fixture
def fake_client(monkeypatch: pytest.MonkeyPatch) -> FakeClient:
    client = FakeClient()

    def get_fake_client() -> FakeClient:
        return client

    monkeypatch.setattr("app.web.get_client", get_fake_client)
    monkeypatch.setattr(realtime, "get_client", sync_client_factory(client))
    return client


def sync_client_factory(client: realtime.SyncClient) -> Callable[..., realtime.SyncClient]:
    def get_sync_client() -> realtime.SyncClient:
        return client

    return get_sync_client


def test_runtime_uses_fake_client(fake_client: FakeClient) -> None:
    run()
    assert fake_client.values == {"key": "value"}
    assert fake_client.messages == [("updates", "payload")]

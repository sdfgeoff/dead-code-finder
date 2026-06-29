import pytest

from pkg.service import run_pipeline, run_provider_pipeline


class FakeMessageSource:
    def read_message(self) -> str:
        return "live"

    def unused_source_helper(self) -> str:
        return "dead"


class FakeMessageSink:
    def __init__(self) -> None:
        self.messages: list[str] = []

    def write_message(self, message: str) -> None:
        self.messages.append(message)

    def unused_sink_helper(self) -> None:
        self.messages.clear()


class FakeProviderSink:
    def __init__(self) -> None:
        self.messages: list[str] = []

    def write_message(self, message: str) -> None:
        self.messages.append(message)

    def unused_provider_sink_helper(self) -> None:
        self.messages.clear()


class FakeSinkProvider:
    def __init__(self) -> None:
        self.sink = FakeProviderSink()

    def open_sink(self) -> FakeProviderSink:
        return self.sink

    def unused_provider_helper(self) -> None:
        self.sink.messages.clear()


@pytest.fixture
def source() -> FakeMessageSource:
    return FakeMessageSource()


@pytest.fixture
def sink() -> FakeMessageSink:
    return FakeMessageSink()


@pytest.fixture
def provider() -> FakeSinkProvider:
    return FakeSinkProvider()


def test_pipeline_transfers_message(
    source: FakeMessageSource,
    sink: FakeMessageSink,
) -> None:
    run_pipeline(source=source, sink=sink)

    assert sink.messages == ["live"]


def test_provider_pipeline_transfers_message(
    source: FakeMessageSource,
    provider: FakeSinkProvider,
) -> None:
    run_provider_pipeline(source=source, provider=provider)

    assert provider.sink.messages == ["live"]

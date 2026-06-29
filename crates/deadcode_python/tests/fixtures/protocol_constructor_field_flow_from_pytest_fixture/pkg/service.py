from typing import Protocol


class MessageSource(Protocol):
    def read_message(self) -> str: ...


class MessageSink(Protocol):
    def write_message(self, message: str) -> None: ...


class SinkProvider(Protocol):
    def open_sink(self) -> MessageSink: ...


class MessagePipeline:
    def __init__(self, source: MessageSource, sink: MessageSink) -> None:
        self.source = source
        self.sink = sink

    def transfer(self) -> None:
        self.sink.write_message(self.source.read_message())


def run_pipeline(source: MessageSource, sink: MessageSink) -> None:
    pipeline = MessagePipeline(source=source, sink=sink)
    pipeline.transfer()


class ProviderPipeline:
    def __init__(self, source: MessageSource, provider: SinkProvider) -> None:
        self.source = source
        self.provider = provider

    def transfer(self) -> None:
        sink = self.provider.open_sink()
        sink.write_message(self.source.read_message())


def run_provider_pipeline(source: MessageSource, provider: SinkProvider) -> None:
    pipeline = ProviderPipeline(source=source, provider=provider)
    pipeline.transfer()

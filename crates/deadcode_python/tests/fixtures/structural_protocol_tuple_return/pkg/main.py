from collections.abc import AsyncIterator
from typing import Protocol


class ChatHistoryProvider(Protocol):
    def get_events(self) -> list[str]: ...

    def append_event(self, event: str) -> None: ...


class History:
    def get_events(self) -> list[str]:
        return []

    def append_event(self, event: str) -> None:
        pass


class WrappedHistory:
    def __init__(self, provider: ChatHistoryProvider) -> None:
        self.provider = provider

    def get_events(self) -> list[str]:
        return self.provider.get_events()

    def append_event(self, event: str) -> None:
        self.provider.append_event(event)


class UnusedHistory:
    def get_events(self) -> list[str]:
        return []

    def append_event(self, event: str) -> None:
        pass


class LLMProvider:
    pass


def build_history() -> tuple[LLMProvider, ChatHistoryProvider]:
    return LLMProvider(), WrappedHistory(History())


async def run(history: ChatHistoryProvider) -> AsyncIterator[str]:
    for event in history.get_events():
        history.append_event(event)
        yield event


async def main() -> None:
    _, history = build_history()
    async for _ in run(history=history):
        pass


main()

from collections.abc import Callable
from typing import Protocol


class PromptContext(Protocol):
    def to_context_prompt(self) -> str: ...


class ChatHistoryProvider(Protocol):
    def get_events(self) -> list[str]: ...

    def append_event(self, event: str) -> None: ...


class ExampleContext:
    def to_context_prompt(self) -> str:
        return "context"


class History:
    def get_events(self) -> list[str]:
        return []

    def append_event(self, event: str) -> None:
        pass


class UnusedHistory:
    def get_events(self) -> list[str]:
        return []

    def append_event(self, event: str) -> None:
        pass


def orchestrate(history: ChatHistoryProvider, frontend_context: PromptContext) -> str:
    for event in history.get_events():
        history.append_event(event)
    return frontend_context.to_context_prompt()


def joiner(stream: Callable[[], str]) -> str:
    return stream()


def main() -> str:
    history = History()
    frontend_context = ExampleContext()
    return joiner(lambda: orchestrate(history, frontend_context))


main()

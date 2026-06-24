from typing import Protocol


class ChatHistoryProvider(Protocol):
    def get_events(self) -> list[str]: ...

    def append_event(self, event: str) -> None: ...


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


def run(history: ChatHistoryProvider) -> None:
    for event in history.get_events():
        history.append_event(event)


run(History())

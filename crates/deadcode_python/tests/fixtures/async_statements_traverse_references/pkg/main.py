from collections.abc import AsyncIterator
from typing import Protocol


class PromptContext(Protocol):
    def to_context_prompt(self) -> str: ...


class ExampleContext:
    def to_context_prompt(self) -> str:
        return "context"


class Client:
    async def __aenter__(self) -> "Client":
        return self

    async def __aexit__(self, exc_type: object, exc: object, tb: object) -> None:
        pass

    def send(self, message: str) -> None:
        pass


class UnusedContext:
    def to_context_prompt(self) -> str:
        return "unused"


class UnusedClient:
    def send(self, message: str) -> None:
        pass


async def stream(frontend_context: PromptContext) -> AsyncIterator[str]:
    yield frontend_context.to_context_prompt()


async def main() -> None:
    frontend_context = ExampleContext()
    async with Client() as client:
        async for message in stream(frontend_context):
            client.send(message)


main()

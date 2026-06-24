from collections.abc import AsyncIterator, Callable


class Choice:
    text: str
    unused: str


class Chunk:
    choices: list[Choice]
    unused: str


async def join(stream: Callable[[], AsyncIterator[Chunk]]) -> str:
    parts: list[str] = []
    async for item in stream():
        if item.choices:
            parts.append(item.choices[0].text)
    return "".join(parts)


join

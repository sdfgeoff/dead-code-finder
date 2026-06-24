from collections.abc import AsyncIterator
from contextlib import asynccontextmanager


class Client:
    def post(self) -> Response:
        return Response()


class Response:
    def json(self) -> dict[str, object]:
        return {}


@asynccontextmanager
async def client() -> AsyncIterator[Client]:
    yield Client()


async def run() -> dict[str, object]:
    async with client() as active:
        response = active.post()
        return response.json()


run()

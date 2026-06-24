import httpx


class Client:
    def __init__(self):
        self.client = httpx.AsyncClient()

    async def fetch(self, url: str) -> object:
        req = await self.client.get(url)
        if req.status_code != 200:
            req.raise_for_status()
        return req.json()


async def post(url: str) -> object:
    async with httpx.AsyncClient() as async_client:
        req = await async_client.post(url)
    if req.status_code != 200:
        return req.status_code
    return req.json()


async def main() -> None:
    await Client().fetch("https://example.com")
    await post("https://example.com")


main()

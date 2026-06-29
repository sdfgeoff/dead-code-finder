from pkg.decorators import cache_items
from pkg.providers import Connection, RedisConnection, TrackedConnection


def create_redis_connection() -> Connection:
    return RedisConnection()


@cache_items(provider=create_redis_connection)
def load_items(keys: list[str]) -> list[str]:
    return keys


def run() -> None:
    load_items(["live"])
    loader = MockLoader()
    cache = TrackedConnection(RedisConnection())
    decorated_loader = cache_items(provider=lambda: cache)(loader)
    decorated_loader(["manual"])


class MockLoader:
    def __call__(self, keys: list[str]) -> list[str]:
        return keys

    def unused(self) -> list[str]:
        return []


run()

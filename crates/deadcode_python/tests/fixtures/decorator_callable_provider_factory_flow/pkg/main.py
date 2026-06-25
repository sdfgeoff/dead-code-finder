from pkg.decorators import cache_items
from pkg.providers import Connection, RedisConnection


def create_redis_connection() -> Connection:
    return RedisConnection()


@cache_items(provider=create_redis_connection)
def load_items(keys: list[str]) -> list[str]:
    return keys


def run() -> None:
    load_items(["live"])


run()

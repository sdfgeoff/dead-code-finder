from typing import Callable

from pkg.providers import Connection, MemoryConnection


class CallableCache[T]:
    def __init__(self, func: Callable[[list[str]], list[str]], provider: Callable[[], Connection] = MemoryConnection):
        self.func = func
        self.cache = provider()

    def get_items(self, keys: list[str]) -> list[bytes | None]:
        return self.cache.lookup(keys)

    def store_items(self, keys: list[str]) -> None:
        self.cache.write(keys)

    def __call__(self, keys: list[str]) -> list[str]:
        cached = self.get_items(keys)
        return self.func([key for key, value in zip(keys, cached) if value is None])

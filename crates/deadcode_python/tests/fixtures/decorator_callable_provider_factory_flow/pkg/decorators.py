from typing import Callable

from pkg.cache import CallableCache
from pkg.providers import Connection, MemoryConnection


def cache_items(provider: Callable[[], Connection] = MemoryConnection):
    def make_decorator(func: Callable[[list[str]], list[str]]):
        return CallableCache[str](func, provider)

    return make_decorator

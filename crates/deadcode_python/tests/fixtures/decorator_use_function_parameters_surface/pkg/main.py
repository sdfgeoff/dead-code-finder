from functools import lru_cache
from typing import Callable, Protocol


class LoaderProtocol(Protocol):
    cache_key: str
    load: Callable[[], list[str]]


class DeadProtocol(Protocol):
    cache_key: str


class Loader:
    cache_key: str

    def load(self) -> list[str]:
        return ["value"]


@lru_cache
def build(loader: LoaderProtocol) -> list[str]:
    return loader.load()


build(Loader())

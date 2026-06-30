from typing import Callable, Protocol

from pkg.types import TValue


class LoaderProtocol(Protocol[TValue]):
    cache_key: str
    load: Callable[[], list[TValue]]


class DeadProtocol(Protocol):
    cache_key: str

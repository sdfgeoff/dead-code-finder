from contextlib import contextmanager
from typing import Generator


class Resource:
    used_field: str
    unused_field: str

    def used(self) -> None:
        _ = self.used_field

    def unused(self) -> None:
        _ = self.unused_field


def get_resource() -> Generator[Resource, None, None]:
    yield Resource()


def run() -> None:
    with contextmanager(get_resource)() as resource:
        resource.used()


run()

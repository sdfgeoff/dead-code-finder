from contextlib import contextmanager
from typing import Generator


class Resource:
    used_field: str
    field_unused: str

    def used(self) -> None:
        _ = self.used_field

    def unused(self) -> None:
        _ = self.field_unused


def get_resource() -> Generator[Resource, None, None]:
    yield Resource()


def run() -> None:
    with contextmanager(get_resource)() as resource:
        resource.used()


run()

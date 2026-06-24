from typing import Generic, Type, TypeVar


T = TypeVar("T")


class Box(Generic[T]):
    item: T


class Model:
    field: str
    unused: str


class Client:
    def fetch(self, model: Type[T]) -> Box[T]:
        raise NotImplementedError


def get_client():
    return Client()

from typing import Generic, Type, TypeVar

from pydantic import BaseModel, TypeAdapter


T = TypeVar("T", bound=BaseModel)


class Envelope(BaseModel, Generic[T]):
    item: T


class Client:
    def parse_payload(self, payload: object, model: Type[T]) -> Envelope[T]:
        parser = TypeAdapter(Envelope[model])
        parsed = parser.validate_python(payload)
        return parsed


def get_client() -> Client:
    return Client()

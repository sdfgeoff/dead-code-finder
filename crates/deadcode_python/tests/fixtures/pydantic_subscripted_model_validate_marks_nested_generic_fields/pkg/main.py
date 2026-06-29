from typing import Generic, TypeVar

from pydantic import BaseModel


T = TypeVar("T")


class Item(BaseModel, Generic[T]):
    value: T


class Envelope(BaseModel, Generic[T]):
    items: list[Item[T]]


class Payload(BaseModel):
    name: str
    owner: str


def load_payload(data: object) -> Envelope[Payload]:
    return Envelope[Payload].model_validate(data)


load_payload({"items": [{"value": {"name": "alpha", "owner": "ops"}}]})

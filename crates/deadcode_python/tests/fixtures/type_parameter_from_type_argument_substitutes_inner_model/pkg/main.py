from typing import Generic, Type, TypeVar

from pydantic import BaseModel, TypeAdapter


T = TypeVar("T", bound=BaseModel)


class Envelope(BaseModel, Generic[T]):
    item: T


class Payload(BaseModel):
    used: str
    parsed_only: str


class DeadPayload(BaseModel):
    dead: str


def parse_payload(payload: object, model: Type[T]) -> Envelope[T]:
    parser = TypeAdapter(Envelope[model])
    return parser.validate_python(payload)


def run(payload: object) -> str:
    parsed = parse_payload(payload, Payload)
    return parsed.item.used


run({})

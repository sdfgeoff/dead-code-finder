from typing import Annotated, Generic, Literal, TypeVar

from pydantic import BaseModel, Field, TypeAdapter


class BaseEvent(BaseModel):
    source_id: str
    unused_base: str


class FirstEvent(BaseEvent):
    event_type: Literal["first"]


class SecondEvent(BaseEvent):
    event_type: Literal["second"]


Event = Annotated[FirstEvent | SecondEvent, Field(discriminator="event_type")]
EventAdapter: TypeAdapter[Event] = TypeAdapter(Event)

T = TypeVar("T", bound=BaseModel)


class Envelope(BaseModel, Generic[T]):
    item: T


class ExternalPayload(BaseModel):
    used_external: str
    parsed_only: str


class DeadPayload(BaseModel):
    dead_external: str


def parse_external(payload: object) -> Envelope[ExternalPayload]:
    parser = TypeAdapter(Envelope[ExternalPayload])
    return parser.validate_python(payload)

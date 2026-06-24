from typing import Annotated, Literal

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

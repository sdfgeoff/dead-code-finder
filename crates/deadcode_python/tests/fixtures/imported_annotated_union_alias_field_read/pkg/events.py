from typing import Annotated, Literal, Union

from pydantic import BaseModel, Field


class FirstEvent(BaseModel):
    event_type: Literal["first"] = "first"
    payload: str
    dead_first: str = "unused"


class SecondEvent(BaseModel):
    event_type: Literal["second"] = "second"
    payload: str
    dead_second: str = "unused"


HistoryEvent = Annotated[
    Union[FirstEvent, SecondEvent],
    Field(discriminator="event_type"),
]

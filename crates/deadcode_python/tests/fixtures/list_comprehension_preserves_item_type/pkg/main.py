from typing import Annotated, Literal, Union
from typing_extensions import TypeAliasType


class PlantingEvent:
    event_type: Literal["planting"]
    species: str | None
    ets_species: str | None
    unused: str


class ClearanceEvent:
    event_type: Literal["clearance"]
    date: str
    unused: str


AnyExampleEvent = TypeAliasType(
    "AnyExampleEvent",
    Annotated[
        Union[
            PlantingEvent,
            ClearanceEvent,
        ],
        "event_type",
    ],
)


def get_example_input_events() -> list[AnyExampleEvent]:
    return []


def needs_region() -> bool:
    all_events = get_example_input_events()
    all_planting_events = [
        event for event in all_events if event.event_type == "planting"
    ]

    for event in all_planting_events:
        if event.ets_species is not None:
            return True
        if event.species is not None:
            return True

    return False


needs_region()

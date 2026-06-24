from typing import NamedTuple


class Event:
    event_type: str
    unused: str


class QueuedEvent(NamedTuple):
    record_id: int
    event: Event


def pop_events() -> list[QueuedEvent]:
    return []


def publish(record_id: int, event: Event) -> None:
    _ = record_id
    _ = event.event_type


def run() -> None:
    for queued_event in pop_events():
        publish(queued_event.record_id, queued_event.event)


run()

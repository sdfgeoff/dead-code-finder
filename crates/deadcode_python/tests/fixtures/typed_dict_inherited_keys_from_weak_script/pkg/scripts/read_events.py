from typing_extensions import TypedDict


class RequiredEvent(TypedDict):
    event_id: str
    timestamp: int
    message: str


class Event(RequiredEvent, total=False):
    source: str
    unused: str


class EventBatch(TypedDict, total=False):
    events: list[Event]
    next_token: str


def collect_events(batch: EventBatch) -> list[str]:
    messages: list[str] = []
    for event in batch.get("events", []):
        messages.append(event["event_id"])
        messages.append(str(event["timestamp"]))
        messages.append(event["message"])
        source = event.get("source")
        if source is not None:
            messages.append(source)
    if batch.get("next_token"):
        messages.append("more")
    return messages


if __name__ == "__main__":
    collect_events({"events": [{"event_id": "evt-1", "timestamp": 1, "message": "ready"}]})

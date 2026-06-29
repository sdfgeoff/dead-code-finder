from pydantic import TypeAdapter
from typing_extensions import TypedDict


class BrokerMessage(TypedDict):
    kind: str
    payload: str | bytes
    unused: str


BrokerMessageAdapter = TypeAdapter(BrokerMessage)


def handle(message: object) -> str:
    parsed = BrokerMessageAdapter.validate_python(message)
    if parsed["kind"] != "event":
        return ""
    raw_payload = parsed["payload"]
    if isinstance(raw_payload, bytes):
        return raw_payload.decode("utf-8")
    return raw_payload


handle({"kind": "event", "payload": "ready"})

from pydantic import BaseModel, TypeAdapter

from pkg.container import Envelope


class Payload(BaseModel):
    used: str
    parsed_only: str


class DeadPayload(BaseModel):
    dead: str


def run(payload: object) -> str:
    parsed = TypeAdapter(Envelope[Payload]).validate_python(payload)
    return parsed.item.used


run({})

from pydantic import BaseModel

from pkg.client import get_client


class Payload(BaseModel):
    used: str
    parsed_only: str


class DeadPayload(BaseModel):
    dead: str


def run(payload: object) -> str:
    parsed = get_client().parse_payload(payload, Payload)
    return parsed.item.used


run({})

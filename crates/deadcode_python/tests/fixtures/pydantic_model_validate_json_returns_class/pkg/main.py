from enum import StrEnum

from pydantic import BaseModel


class Status(StrEnum):
    READY = "ready"
    WAITING = "waiting"


class Details(BaseModel):
    status: Status
    nested_value: str


class Args(BaseModel):
    answer: str
    unused: str
    details: Details


def run(payload: str) -> str:
    args = Args.model_validate_json(payload)
    return args.answer


run("{}")

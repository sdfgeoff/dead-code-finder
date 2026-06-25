from pydantic import BaseModel

from pkg.framework import Router

router = Router()


class Nested(BaseModel):
    only_none: None = None


class Payload(BaseModel):
    absent: None = None
    nested: Nested


class DeadPayload(BaseModel):
    absent: None = None


@router.post("")
def create_item(payload: Payload) -> Payload:
    return payload

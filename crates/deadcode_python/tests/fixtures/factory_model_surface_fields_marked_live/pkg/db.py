from typing import Annotated, Union

from pkg.store import query_returning_one


class InputRow:
    user_id: int
    tenant_id: int


class OutputRow:
    id: int
    name: str
    extra: str


class DeadRow:
    value: str


class FirstEvent:
    kind: str
    payload: str


class SecondEvent:
    kind: str
    code: int


EventRow = Annotated[Union[FirstEvent, SecondEvent], object()]


get_user = query_returning_one(
    input=InputRow,
    output=OutputRow,
    sql="SELECT id, name, extra FROM entity WHERE user_id = :user_id",
)

get_event = query_returning_one(
    input=InputRow,
    output=EventRow,
    sql="SELECT kind, payload, code FROM event WHERE user_id = :user_id",
)

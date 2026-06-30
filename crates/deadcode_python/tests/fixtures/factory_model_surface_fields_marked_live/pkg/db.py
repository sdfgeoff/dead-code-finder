from typing import Annotated, Union

from pkg.store import query_returning_list, query_returning_one


class InputRow:
    user_id: int
    tenant_id: int


class OutputRow:
    id: int
    name: str
    extra: str


class DeadRow:
    value: str


class PositionalInput:
    user_id: int
    record_id: int


class PositionalOutput:
    user_id: int
    group_id: int
    record_id: int


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

get_associations = query_returning_list(
    PositionalInput,
    PositionalOutput,
    "SELECT user_id, group_id, record_id FROM user_group_record",
)

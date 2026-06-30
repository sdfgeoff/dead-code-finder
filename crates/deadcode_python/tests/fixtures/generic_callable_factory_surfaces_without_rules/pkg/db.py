from typing import Callable, Optional, TypeVar

from pkg.store import query_optional, query_batch_none, query_many


class InputRow:
    required: int
    serialized_only: str


class OutputRow:
    id: int
    constructed_only: str


class BatchInput:
    item_id: int
    payload: str


class DeadRow:
    value: str


get_one = query_optional(
    input=InputRow,
    output=OutputRow,
    sql="SELECT id, constructed_only FROM rows WHERE required = :required",
)

get_many = query_many(
    input=InputRow,
    output=OutputRow,
    sql="SELECT id, constructed_only FROM rows",
)

save_batch = query_batch_none(
    input=BatchInput,
    sql="UPDATE rows SET payload = :payload WHERE item_id = :item_id",
)


TIn = TypeVar("TIn")
TOut = TypeVar("TOut")


def local_query_optional(
    *,
    input: type[TIn],
    output: type[TOut],
    sql: str,
) -> Callable[[object, TIn], Optional[TOut]]:
    raise NotImplementedError(sql)

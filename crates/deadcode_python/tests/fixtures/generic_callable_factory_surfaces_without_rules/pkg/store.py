from typing import Callable, List, Optional, TypeVar


TIn = TypeVar("TIn")
TOut = TypeVar("TOut")


def query_optional(
    *,
    input: type[TIn],
    output: type[TOut],
    sql: str,
) -> Callable[[object, TIn], Optional[TOut]]:
    raise NotImplementedError(sql)


def query_many(
    *,
    input: type[TIn],
    output: type[TOut],
    sql: str,
) -> Callable[[object, TIn], List[TOut]]:
    raise NotImplementedError(sql)


def query_batch_none(
    *,
    input: type[TIn],
    sql: str,
) -> Callable[[object, List[TIn]], None]:
    raise NotImplementedError(sql)

from typing import Callable, TypeVar


InputT = TypeVar("InputT")
OutputT = TypeVar("OutputT")


def query_returning_one(
    *,
    input: type[InputT],
    output: type[OutputT],
    sql: str,
) -> Callable[[object, InputT], OutputT]:
    raise NotImplementedError()


def query_returning_list(
    input: type[InputT],
    output: type[OutputT],
    sql: str,
) -> Callable[[object, InputT], list[OutputT]]:
    raise NotImplementedError()

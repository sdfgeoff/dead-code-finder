from typing import TypeVar

TModel = TypeVar("TModel")


def parse(model: type[TModel], data: object) -> TModel: ...


def dead_helper() -> None: ...

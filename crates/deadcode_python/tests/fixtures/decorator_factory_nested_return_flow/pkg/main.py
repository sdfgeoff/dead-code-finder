from functools import wraps
from typing import Any, Callable, TypeVar, cast


FuncT = TypeVar("FuncT", bound=Callable[..., Any])


def deprecated(replacement: str) -> Callable[[FuncT], FuncT]:
    def decorator(func: FuncT) -> FuncT:
        @wraps(func)
        def inner(*args: Any, **kwargs: Any) -> Any:
            print(f"use {replacement}")
            return func(*args, **kwargs)

        return cast(FuncT, inner)

    return decorator


@deprecated("new_endpoint")
def old_endpoint() -> str:
    return "old"


def new_endpoint() -> str:
    return "new"


def main() -> None:
    print(old_endpoint())


if __name__ == "__main__":
    main()

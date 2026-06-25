from typing import Callable, TypeVar

F = TypeVar("F", bound=Callable[..., object])


class Router:
    def post(self, path: str) -> Callable[[F], F]:
        def decorator(func: F) -> F:
            return func

        return decorator

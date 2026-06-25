from typing import Callable

from pkg.cache import CallableCache


def cache_array(prefix: str) -> Callable[[Callable[[list[int]], list[int]]], Callable[[list[int]], list[int]]]:
    def make_decorator(func: Callable[[list[int]], list[int]]) -> Callable[[list[int]], list[int]]:
        cache_wrapper = CallableCache(func)

        def wrapper(inputs: list[int]) -> list[int]:
            return cache_wrapper(inputs)

        return wrapper

    return make_decorator

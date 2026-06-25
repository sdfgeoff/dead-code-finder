from typing import Callable


class CallableCache:
    def __init__(self, func: Callable[[list[int]], list[int]]) -> None:
        self.func = func

    def helper(self, inputs: list[int]) -> list[int]:
        return self.func(inputs)

    def __call__(self, inputs: list[int]) -> list[int]:
        return self.helper(inputs)


class UnusedCallableCache:
    def __call__(self, inputs: list[int]) -> list[int]:
        return inputs

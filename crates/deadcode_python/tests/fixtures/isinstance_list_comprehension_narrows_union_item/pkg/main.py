from typing import Generic, TypeVar


TInput = TypeVar("TInput")
TResult = TypeVar("TResult")


class CacheHit(Generic[TResult]):
    key: str
    result: TResult
    unused: str


class CacheMiss(Generic[TInput]):
    key: str
    input: TInput
    unused: str


class Input:
    value: int
    unused: str


def load_items() -> list[CacheHit[str] | CacheMiss[Input]]:
    return []


def compute(inputs: list[Input]) -> list[str]:
    return [str(item.value) for item in inputs]


def run() -> list[CacheHit[str]]:
    cache_results = load_items()
    misses = [item for item in cache_results if isinstance(item, CacheMiss)]
    computed = compute([miss.input for miss in misses])
    return [
        CacheHit(key=miss.key, result=computed[idx])
        for idx, miss in enumerate(misses)
    ]


run()

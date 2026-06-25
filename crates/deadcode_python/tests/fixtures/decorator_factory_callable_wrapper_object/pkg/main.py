from pkg.decorators import cache_array


@cache_array(prefix="items")
def compute(inputs: list[int]) -> list[int]:
    return [item + 1 for item in inputs]


compute([1])

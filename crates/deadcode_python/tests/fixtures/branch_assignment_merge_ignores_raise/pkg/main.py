class Result:
    values: list[int]
    unused: str


def zero() -> Result:
    return Result()


def nonzero() -> Result:
    return Result()


def run(count: int) -> list[int]:
    if count == 0:
        result = zero()
    elif count == 1:
        result = nonzero()
    else:
        raise ValueError("bad count")

    return result.values


run(1)

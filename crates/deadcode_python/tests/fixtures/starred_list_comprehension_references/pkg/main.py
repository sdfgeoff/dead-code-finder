def used(value: int) -> int:
    return value


def dead(value: int) -> int:
    return value


def sink(*items: int) -> None:
    pass


def run(values: list[int]) -> None:
    sink(*[used(value) for value in values])


run([1])

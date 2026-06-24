class Accumulator:
    id: str | None
    name: str | None
    unused: str


def run():
    accumulators: dict[int, Accumulator] = {}
    accum = accumulators.setdefault(1, Accumulator())
    if accum.id:
        return accum.name
    return None


run()

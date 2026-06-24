from pkg.aliases import ExternalShape


class Props:
    value: str


def other_shape() -> ExternalShape:
    raise NotImplementedError


def run(other: ExternalShape) -> bool:
    def entries() -> list[tuple[ExternalShape, Props]]:
        raise NotImplementedError

    all_entries = entries()
    geom, props = all_entries[0]
    return not geom.intersects(other)


run(other_shape())

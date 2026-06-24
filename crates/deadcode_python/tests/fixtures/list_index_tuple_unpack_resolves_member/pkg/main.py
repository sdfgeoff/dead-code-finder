class Geometry:
    def intersects(self, other: "Geometry") -> bool:
        return True

    def unused(self) -> bool:
        return False


class Props:
    value: str


def entries() -> list[tuple[Geometry, Props]]:
    return [(Geometry(), Props())]


def run(other: Geometry) -> bool:
    all_entries = entries()
    geom, props = all_entries[0]
    return not geom.intersects(other)


run(Geometry())

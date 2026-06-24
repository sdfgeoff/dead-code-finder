class Bound:
    minx: float
    maxx: float
    unused: float

    def overlaps(self, other: "Bound") -> bool:
        return self.minx < other.maxx and other.minx < self.maxx


Bound().overlaps(Bound())

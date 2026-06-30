def example_tag(value: str) -> str:
    return f"[{value}]"


def dead_tag(value: str) -> str:
    return f"<{value}>"


class Series:
    def apply(self, mapper):
        return mapper(["a", "b"])


def main() -> None:
    series = Series()
    series.apply(lambda values: "".join(example_tag(value) for value in values))


main()

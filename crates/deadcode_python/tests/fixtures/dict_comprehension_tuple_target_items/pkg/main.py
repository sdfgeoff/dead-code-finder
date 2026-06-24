class Source:
    live: str
    unused: str


def source_by_name() -> dict[str, Source]:
    return {}


def run(names: list[str]) -> list[str]:
    filtered = {k: v for k, v in source_by_name().items() if k in names}
    return [source.live for _, source in filtered.items()]


run(["a"])

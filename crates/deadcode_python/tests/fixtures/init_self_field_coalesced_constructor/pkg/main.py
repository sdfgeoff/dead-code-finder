class Cache:
    def get(self) -> list[Feature]:
        return [Feature()]


class Feature:
    value: str
    unused: str


class Tool:
    def __init__(self, cache: Cache | None = None) -> None:
        self.cache = cache or Cache()

    def execute(self) -> str:
        features = self.cache.get()
        return features[0].value


def run() -> str:
    return Tool().execute()


run()

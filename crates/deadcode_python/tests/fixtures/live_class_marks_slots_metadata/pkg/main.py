class LiveBucket:
    __slots__ = ("value",)

    def __init__(self) -> None:
        self.value: int = 1


class DeadBucket:
    __slots__ = ("value",)

    def __init__(self) -> None:
        self.value: int = 1


def run() -> int:
    bucket = LiveBucket()
    return bucket.value


run()

class LiveError(Exception):
    pass


class CauseError(Exception):
    pass


class UnusedError(Exception):
    pass


def fail(value: int) -> None:
    if value > 0:
        raise LiveError("bad value") from CauseError("cause")


fail(1)

class Event:
    pass


class LivePattern:
    pass


class UnusedPattern:
    pass


def guard(value: object) -> bool:
    return value is not None


def live_body() -> None:
    return None


def unused_body() -> None:
    return None


def handle(event: object) -> None:
    match event:
        case LivePattern() if guard(event):
            live_body()
        case _:
            pass


handle(Event())

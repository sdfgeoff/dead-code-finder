from pkg.events import FirstEvent, HistoryEvent


def describe(event: HistoryEvent) -> str:
    return str(event.event_type)


def run() -> str:
    return describe(FirstEvent(payload="ready"))


run()

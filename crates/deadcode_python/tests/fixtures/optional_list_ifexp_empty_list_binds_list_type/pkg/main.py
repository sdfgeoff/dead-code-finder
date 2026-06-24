class ExampleMessage:
    to: str
    unused: str


class Process:
    message_items: list[ExampleMessage] | None
    unused: str


def complete(process: Process) -> str:
    message_items = process.message_items if process.message_items is not None else []
    message_items.append(ExampleMessage())
    return message_items[0].to


complete(Process())

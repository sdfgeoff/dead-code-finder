class Item:
    value: int


class Wrapped:
    item: Item | None


class TextResult:
    content: str


class ToolFunction:
    name: str


class ToolCall:
    function: ToolFunction


class ToolResult:
    tool_calls: list[ToolCall]


StepResult = TextResult | ToolResult


def to_items(values: list[int]) -> list[Item]:
    return [Item(value=value) for value in values]


def score(item: Item) -> int:
    return item.value


def dead(item: Item) -> int:
    return item.value


def total(values: list[int]) -> int:
    return sum(score(item) for item in to_items(values))


def wrapped_items(values: list[int]) -> list[Wrapped]:
    return [Wrapped(item=item) for item in to_items(values)]


def total_wrapped(values: list[int]) -> int:
    generated = (wrapped.item for wrapped in wrapped_items(values))
    return sum(score(item) for item in generated if item is not None)


def total_concatenated(first: list[Item], second: list[Item]) -> int:
    combined = first + second
    return sum(item.value for item in combined)


def stop_name() -> str:
    return "stop"


def tool_name(result: StepResult) -> str | None:
    if isinstance(result, TextResult):
        return None
    stop = next(
        (tool_call for tool_call in result.tool_calls if tool_call.function.name == stop_name()),
        None,
    )
    if stop is None:
        return None
    return stop.function.name


total([1])
total_wrapped([1])
total_concatenated(to_items([1]), to_items([2]))
tool_name(ToolResult(tool_calls=[]))

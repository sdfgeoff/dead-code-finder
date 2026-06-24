from collections.abc import Callable

from lib.type_defs import PromptContext


def orchestrate(*, frontend_context: PromptContext) -> str:
    return frontend_context.to_context_prompt()


def stream_joiner(stream: Callable[[], str]) -> str:
    return stream()

from typing import Protocol


class PromptContext(Protocol):
    def to_context_prompt(self) -> str: ...


class ExampleContext:
    def to_context_prompt(self) -> str:
        return "live"


class UnusedContext:
    def to_context_prompt(self) -> str:
        return "dead"


def setup(frontend_context: PromptContext) -> str:
    return frontend_context.to_context_prompt()


def orchestrate(frontend_context: PromptContext) -> str:
    return setup(frontend_context)


def run() -> str:
    frontend_context = ExampleContext()
    return orchestrate(frontend_context)


run()

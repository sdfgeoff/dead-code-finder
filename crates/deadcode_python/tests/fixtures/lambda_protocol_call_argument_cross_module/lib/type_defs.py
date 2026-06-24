from typing import Protocol


class PromptContext(Protocol):
    def to_context_prompt(self) -> str: ...

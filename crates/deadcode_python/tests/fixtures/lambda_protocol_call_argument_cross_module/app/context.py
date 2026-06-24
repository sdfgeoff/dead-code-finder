class ExampleContext:
    resource_id: int

    def __init__(self, resource_id: int) -> None:
        self.resource_id = resource_id

    def to_context_prompt(self) -> str:
        return f"resource_id={self.resource_id}"

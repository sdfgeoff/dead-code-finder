from pydantic import BaseModel, ConfigDict


class ExampleContext(BaseModel):
    model_config = ConfigDict(extra="forbid")

    resource_id: int

    def to_context_prompt(self) -> str:
        return f"resource_id={self.resource_id}"

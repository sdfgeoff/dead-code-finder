from typing import Annotated, Literal

from pydantic import BaseModel, Field


class EmailTask(BaseModel):
    item_kind: Literal["field_text"]
    subject: str


class ExampleFileItem(BaseModel):
    item_kind: Literal["file"]
    path: str


ExampleItem = Annotated[EmailTask | ExampleFileItem, Field(discriminator="item_kind")]


class Envelope(BaseModel):
    item: ExampleItem
    unused: str


def parse_item(data: dict[str, object]) -> ExampleItem:
    return Envelope.model_validate({"item": data}).item


parse_item({"item_kind": "field_text", "subject": "hello"})

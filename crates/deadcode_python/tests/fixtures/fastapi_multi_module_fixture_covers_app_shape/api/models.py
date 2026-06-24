from pydantic import BaseModel


class ExampleEntity(BaseModel):
    name: str
    field_text: str

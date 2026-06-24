from pydantic import BaseModel, field_validator


class Model(BaseModel):
    value: str

    @field_validator("value")
    @classmethod
    def normalize(cls, value: object) -> str:
        return value if isinstance(value, str) else ""

    def dead(self) -> None:
        pass


Model(value="ok")

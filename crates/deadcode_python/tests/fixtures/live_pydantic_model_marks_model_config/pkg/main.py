from pydantic import BaseModel, ConfigDict


class LiveModel(BaseModel):
    model_config = ConfigDict(extra="forbid")
    value: str


class DeadModel(BaseModel):
    model_config = ConfigDict(extra="forbid")
    value: str


def run(payload: str) -> str:
    model = LiveModel.model_validate_json(payload)
    return model.value


run("{}")

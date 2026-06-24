from pydantic import BaseModel


class Args(BaseModel):
    answer: str
    unused: str


def run(payload: str) -> str:
    args = Args.model_validate_json(payload)
    return args.answer


run("{}")

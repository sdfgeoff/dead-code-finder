from typing import Literal

from pydantic import BaseModel


class ThirdTemplate(BaseModel):
    template_type: Literal["third"] = "third"
    detail: str

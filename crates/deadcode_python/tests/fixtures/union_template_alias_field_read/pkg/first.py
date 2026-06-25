from typing import Literal

from pydantic import BaseModel


class FirstTemplate(BaseModel):
    template_type: Literal["first"] = "first"
    label: str
    unused: str = ""

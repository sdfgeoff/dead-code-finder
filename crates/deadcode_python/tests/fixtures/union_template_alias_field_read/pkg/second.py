from typing import Literal

from pydantic import BaseModel


class SecondTemplate(BaseModel):
    template_type: Literal["second"] = "second"
    body: str

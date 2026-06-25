from typing import TypeVar

from pydantic import BaseModel


PayloadType = TypeVar("PayloadType", bound=BaseModel)

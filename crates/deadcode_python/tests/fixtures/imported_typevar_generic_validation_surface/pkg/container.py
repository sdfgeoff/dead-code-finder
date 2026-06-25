from typing import Generic

from pydantic import BaseModel

from pkg.typevars import PayloadType


class Envelope(BaseModel, Generic[PayloadType]):
    item: PayloadType
